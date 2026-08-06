use mlua::{Lua, LuaSerdeExt};
use serde::{Deserialize, Serialize};
use std::{fs, net::IpAddr, path::Path};
use tokio::time::{timeout, Duration};

pub mod errors;
pub mod sandbox;
pub mod secrets;

use errors::{classify_lua_error, SandboxError};

// ── Constants ────────────────────────────────────────────────────────────────

/// Wall-clock timeout for a single provider search call (Lua + HTTP combined).
const SEARCH_TIMEOUT_SECS: u64 = 15;

/// Maximum number of bytes we will buffer from a single HTTP response.
/// Prevents a malicious server from OOM-ing the process via a Lua script.
const HTTP_RESPONSE_MAX_BYTES: usize = 1024 * 1024; // 1 MB

// ── Public types ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct TrackResult {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub cover_art_url: Option<String>,
    pub stream_url: Option<String>,
    pub quality_hint: Option<String>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResolvedTrack {
    pub stream_url: String,
    pub quality_hint: Option<String>,
    pub duration_ms: Option<u64>,
}

pub struct ActiveProvider {
    pub id: String,
    pub name: String,
    pub script_path: std::path::PathBuf,
    pub config: std::collections::HashMap<String, String>,
}

pub struct ProviderManager {
    providers: std::collections::HashMap<String, ActiveProvider>,
    reqwest_client: reqwest::Client,
}

// ── SSRF guard ───────────────────────────────────────────────────────────────

/// Returns an error string if the URL targets a private/loopback/link-local
/// address, preventing Server-Side Request Forgery from malicious scripts.
pub fn check_url_allowed(url: &str) -> Result<(), SandboxError> {
    let parsed = url::Url::parse(url).map_err(|e| SandboxError::ForbiddenUrl {
        url: url.to_string(),
        reason: format!("invalid URL: {}", e),
    })?;

    // Only allow HTTP and HTTPS
    match parsed.scheme() {
        "http" | "https" => {}
        scheme => {
            return Err(SandboxError::ForbiddenUrl {
                url: url.to_string(),
                reason: format!("scheme '{}' not allowed (only http/https)", scheme),
            })
        }
    }

    // Resolve host and check for private/loopback addresses
    if let Some(host) = parsed.host_str() {
        // Check if it's a raw IP address
        if let Ok(ip) = host.parse::<IpAddr>() {
            if is_ip_forbidden(&ip) {
                return Err(SandboxError::ForbiddenUrl {
                    url: url.to_string(),
                    reason: format!("IP address '{}' targets a private/loopback range", ip),
                });
            }
        }
        // Block known dangerous hostnames
        if host == "localhost" || host.ends_with(".local") {
            return Err(SandboxError::ForbiddenUrl {
                url: url.to_string(),
                reason: format!("hostname '{}' resolves to a local address", host),
            });
        }
    }

    Ok(())
}

pub fn is_ip_forbidden(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()           // 127.0.0.0/8
                || v4.is_private()     // 10/8, 172.16/12, 192.168/16
                || v4.is_link_local()  // 169.254.0.0/16 (AWS metadata)
                || v4.is_broadcast()
                || v4.is_unspecified()
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()           // ::1
                || v6.is_unspecified() // ::
        }
    }
}

// ── ProviderManager ──────────────────────────────────────────────────────────

impl ProviderManager {
    pub fn new(reqwest_client: reqwest::Client) -> Self {
        Self {
            providers: std::collections::HashMap::new(),
            reqwest_client,
        }
    }

    pub fn sync_registry(&mut self, providers_info: Vec<crate::ProviderInfo>) {
        let mut new_registry = std::collections::HashMap::new();
        for info in providers_info {
            if info.status != "enabled" {
                continue;
            }
            let config = if let Some(settings) = info.settings {
                serde_json::from_str(&settings).unwrap_or_default()
            } else {
                std::collections::HashMap::new()
            };
            new_registry.insert(info.id.clone(), ActiveProvider {
                id: info.id,
                name: info.name,
                script_path: std::path::PathBuf::from(info.file_path),
                config,
            });
        }
        self.providers = new_registry;
        log::info!("Provider registry synced with {} enabled providers", self.providers.len());
    }

    /// Execute the provider's `search` function inside a fresh, sandboxed
    /// Lua VM.  The entire call (Lua init + HTTP + result parsing) is bounded
    /// by `SEARCH_TIMEOUT_SECS`.
    pub async fn search(
        &self,
        provider_id: &str,
        query: &str,
    ) -> Result<Vec<TrackResult>, SandboxError> {
        let provider = self.providers.get(provider_id).ok_or_else(|| SandboxError::ScriptError {
            script: provider_id.to_string(),
            message: "provider not found or disabled in registry".to_string(),
        })?;

        let script_path = provider.script_path.clone();
        let script_name = provider.name.clone();
        let config = provider.config.clone();
        let query = query.to_string();

        let fut_script_name = script_name.clone();
        let client = self.reqwest_client.clone();
        let fut = async move {
            run_search(&script_path, &fut_script_name, &query, client, config).await
        };

        timeout(Duration::from_secs(SEARCH_TIMEOUT_SECS), fut)
            .await
            .map_err(|_| {
                crate::telemetry::record_error(
                    "lua_sandbox",
                    &format!("search timeout in provider '{}'", script_name),
                );
                SandboxError::ExecutionTimeout {
                    script: script_name.clone(),
                    timeout_secs: SEARCH_TIMEOUT_SECS,
                }
            })?
    }

    /// Execute the provider's `resolve` function to get a short-lived streaming URL
    /// for a specific track ID.
    pub async fn resolve(
        &self,
        provider_id: &str,
        track_id: &str,
    ) -> Result<ResolvedTrack, SandboxError> {
        let provider = self.providers.get(provider_id).ok_or_else(|| SandboxError::ScriptError {
            script: provider_id.to_string(),
            message: "provider not found or disabled in registry".to_string(),
        })?;

        let script_path = provider.script_path.clone();
        let script_name = provider.name.clone();
        let config = provider.config.clone();
        let track_id = track_id.to_string();
        let client = self.reqwest_client.clone();

        let fut_script_name = script_name.clone();
        let fut = async move {
            run_resolve(&script_path, &fut_script_name, &track_id, client, config).await
        };

        timeout(Duration::from_secs(SEARCH_TIMEOUT_SECS), fut)
            .await
            .map_err(|_| {
                crate::telemetry::record_error(
                    "lua_sandbox",
                    &format!("resolve timeout in provider '{}'", script_name),
                );
                SandboxError::ExecutionTimeout {
                    script: script_name.clone(),
                    timeout_secs: SEARCH_TIMEOUT_SECS,
                }
            })?
    }
}

/// Spin up a fresh Lua VM, configure the sandbox, inject globals,
/// load the script, and call `provider.search(query)`.
async fn run_search(
    script_path: &Path,
    script_name: &str,
    query: &str,
    client: reqwest::Client,
    config: std::collections::HashMap<String, String>,
) -> Result<Vec<TrackResult>, SandboxError> {
    // ── Fresh VM ─────────────────────────────────────────────────────────────
    let lua = Lua::new();
    sandbox::configure_sandbox(&lua, script_name)
        .map_err(|e| classify_lua_error(e, script_name))?;

    // ── Inject http.* table ──────────────────────────────────────────────────
    inject_http_table(&lua, client, script_name)?;

    // ── Inject provider_config (read-only) ───────────────────────────────────
    let config_table = lua.create_table().map_err(|e| classify_lua_error(e, script_name))?;
    for (key, value) in &config {
        config_table.set(key.as_str(), value.as_str()).map_err(|e| classify_lua_error(e, script_name))?;
    }
    lua.globals().set("provider_config", config_table).map_err(|e| classify_lua_error(e, script_name))?;

    // ── Inject JSON decoding ─────────────────────────────────────────────────
    let json_table = lua.create_table().map_err(|e| classify_lua_error(e, script_name))?;
    let decode_func = lua.create_function(|lua, string: String| {
        let v: serde_json::Value = serde_json::from_str(&string).map_err(mlua::Error::external)?;
        lua.to_value(&v).map_err(mlua::Error::external)
    }).map_err(|e| classify_lua_error(e, script_name))?;
    json_table.set("decode", decode_func).map_err(|e| classify_lua_error(e, script_name))?;
    lua.globals().set("json", json_table).map_err(|e| classify_lua_error(e, script_name))?;

    // ── Load Script ───────────────────────────────────────────────────────────
    let script_content = fs::read_to_string(script_path).map_err(|e| SandboxError::ScriptError {
        script: script_name.to_string(),
        message: e.to_string(),
    })?;

    let provider_table: mlua::Table = lua
        .load(&script_content)
        .eval()
        .map_err(|e| classify_lua_error(e, script_name))?;

    let search_fn: mlua::Function = provider_table
        .get("search")
        .map_err(|e| classify_lua_error(e, script_name))?;

    // ── Call search ───────────────────────────────────────────────────────────
    let lua_value: mlua::Value = search_fn
        .call_async(query)
        .await
        .map_err(|e| classify_lua_error(e, script_name))?;

    let results: Vec<TrackResult> = lua
        .from_value(lua_value)
        .map_err(|e| classify_lua_error(e, script_name))?;

    Ok(results)
}

/// Spin up a fresh Lua VM, configure the sandbox, inject globals,
/// load the script, and call `provider.resolve(track_id)`.
async fn run_resolve(
    script_path: &Path,
    script_name: &str,
    track_id: &str,
    client: reqwest::Client,
    config: std::collections::HashMap<String, String>,
) -> Result<ResolvedTrack, SandboxError> {
    let lua = Lua::new();
    sandbox::configure_sandbox(&lua, script_name)
        .map_err(|e| classify_lua_error(e, script_name))?;

    inject_http_table(&lua, client, script_name)?;

    let config_table = lua.create_table().map_err(|e| classify_lua_error(e, script_name))?;
    for (key, value) in &config {
        config_table.set(key.as_str(), value.as_str()).map_err(|e| classify_lua_error(e, script_name))?;
    }
    lua.globals().set("provider_config", config_table).map_err(|e| classify_lua_error(e, script_name))?;

    let json_table = lua.create_table().map_err(|e| classify_lua_error(e, script_name))?;
    let decode_func = lua.create_function(|lua, string: String| {
        let v: serde_json::Value = serde_json::from_str(&string).map_err(mlua::Error::external)?;
        lua.to_value(&v).map_err(mlua::Error::external)
    }).map_err(|e| classify_lua_error(e, script_name))?;
    json_table.set("decode", decode_func).map_err(|e| classify_lua_error(e, script_name))?;
    lua.globals().set("json", json_table).map_err(|e| classify_lua_error(e, script_name))?;

    let script_content = fs::read_to_string(script_path).map_err(|e| SandboxError::ScriptError {
        script: script_name.to_string(),
        message: e.to_string(),
    })?;

    let provider_table: mlua::Table = lua
        .load(&script_content)
        .eval()
        .map_err(|e| classify_lua_error(e, script_name))?;

    let resolve_fn: mlua::Function = provider_table
        .get("resolve")
        .map_err(|e| classify_lua_error(e, script_name))?;

    let lua_value: mlua::Value = resolve_fn
        .call_async(track_id)
        .await
        .map_err(|e| classify_lua_error(e, script_name))?;

    let resolved: ResolvedTrack = lua
        .from_value(lua_value)
        .map_err(|e| classify_lua_error(e, script_name))?;

    Ok(resolved)
}

// ── Shared HTTP Request Infrastructure ──────────────────────────────────────

/// Headers that Lua scripts are never allowed to set.
/// Prevents request smuggling and host spoofing from sandboxed code.
const FORBIDDEN_HEADERS: &[&str] = &["host", "transfer-encoding", "content-length"];

/// Core HTTP dispatcher shared by all `http.*` Lua functions.
/// Enforces SSRF guards, forbidden header stripping, and response size caps.
async fn do_http_request(
    client: &reqwest::Client,
    method: reqwest::Method,
    url: &str,
    headers: Option<std::collections::HashMap<String, String>>,
    body: Option<String>,
) -> Result<String, mlua::Error> {
    check_url_allowed(url).map_err(mlua::Error::external)?;

    let mut request = client.request(method, url);

    // Apply user-supplied headers, stripping forbidden ones
    if let Some(hdrs) = headers {
        for (key, value) in hdrs {
            if FORBIDDEN_HEADERS.contains(&key.to_lowercase().as_str()) {
                continue; // Silently strip
            }
            request = request.header(&key, &value);
        }
    }

    // Attach body for POST/PUT methods
    if let Some(b) = body {
        request = request.body(b);
    }

    let response = request.send().await.map_err(mlua::Error::external)?;

    let raw_bytes = response.bytes().await.map_err(mlua::Error::external)?;

    if raw_bytes.len() > HTTP_RESPONSE_MAX_BYTES {
        let err = SandboxError::ResponseTooLarge {
            url: url.to_string(),
            limit_bytes: HTTP_RESPONSE_MAX_BYTES,
        };
        crate::telemetry::record_error("lua_sandbox", &err.to_string());
        return Err(mlua::Error::external(err));
    }

    String::from_utf8(raw_bytes.to_vec()).map_err(mlua::Error::external)
}

/// Build and inject the `http` Lua global table with `get`, `get_with_headers`,
/// and `post` functions. All share the same SSRF/size-cap infrastructure.
fn inject_http_table(
    lua: &Lua,
    client: reqwest::Client,
    script_name: &str,
) -> Result<(), SandboxError> {
    let http_table = lua.create_table().map_err(|e| classify_lua_error(e, script_name))?;

    // ── http.get(url) → string ──────────────────────────────────────────────
    let client_get = client.clone();
    let get_func = lua
        .create_async_function(move |_lua, url: String| {
            let client = client_get.clone();
            async move {
                do_http_request(&client, reqwest::Method::GET, &url, None, None).await
            }
        })
        .map_err(|e| classify_lua_error(e, script_name))?;

    // ── http.get_with_headers(url, headers_table) → string ──────────────────
    let client_gwh = client.clone();
    let get_with_headers_func = lua
        .create_async_function(move |_lua, (url, headers): (String, mlua::Table)| {
            let client = client_gwh.clone();
            let mut header_map = std::collections::HashMap::new();
            for pair in headers.pairs::<String, String>() {
                if let Ok((k, v)) = pair {
                    header_map.insert(k, v);
                }
            }
            async move {
                do_http_request(&client, reqwest::Method::GET, &url, Some(header_map), None).await
            }
        })
        .map_err(|e| classify_lua_error(e, script_name))?;

    // ── http.post(url, body, headers_table?) → string ───────────────────────
    let client_post = client;
    let post_func = lua
        .create_async_function(
            move |_lua, (url, body, headers): (String, String, Option<mlua::Table>)| {
                let client = client_post.clone();
                let header_map = headers.map(|tbl| {
                    let mut map = std::collections::HashMap::new();
                    for pair in tbl.pairs::<String, String>() {
                        if let Ok((k, v)) = pair {
                            map.insert(k, v);
                        }
                    }
                    map
                });
                async move {
                    do_http_request(
                        &client,
                        reqwest::Method::POST,
                        &url,
                        header_map,
                        Some(body),
                    )
                    .await
                }
            },
        )
        .map_err(|e| classify_lua_error(e, script_name))?;

    http_table.set("get", get_func).map_err(|e| classify_lua_error(e, script_name))?;
    http_table.set("get_with_headers", get_with_headers_func).map_err(|e| classify_lua_error(e, script_name))?;
    http_table.set("post", post_func).map_err(|e| classify_lua_error(e, script_name))?;

    lua.globals().set("http", http_table).map_err(|e| classify_lua_error(e, script_name))?;

    Ok(())
}

#[cfg(test)]
mod sandbox_tests;
