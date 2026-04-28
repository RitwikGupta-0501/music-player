use mlua::{Function, Lua, LuaSerdeExt, Table};
use serde::{Deserialize, Serialize};
use std::fs;

// 1. Define the Rust struct that mirrors the Lua output
#[derive(Debug, Serialize, Deserialize)]
pub struct TrackResult {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub stream_url: String,
}

pub struct ProviderManager {
    lua: Lua,
}

impl ProviderManager {
    // 2. Initialize the Sandbox and Inject the HTTP Client
    pub fn new() -> mlua::Result<Self> {
        let lua = Lua::new();

        // Create a global 'http' table in Lua
        let http_table = lua.create_table()?;

        // Create the async Rust function that Lua will call
        let get_func = lua.create_async_function(|_lua, url: String| async move {
            // Use reqwest to make the network call
            let res = reqwest::get(&url)
                .await
                .map_err(|e| mlua::Error::external(e))?;

            let text = res.text().await.map_err(|e| mlua::Error::external(e))?;

            Ok(text)
        })?;

        // Attach the function to the table: http.get = get_func
        http_table.set("get", get_func)?;
        lua.globals().set("http", http_table)?;

        Ok(Self { lua })
    }

    // 3. Load the script and execute the 'search' method
    pub async fn search(&self, script_path: &str, query: &str) -> mlua::Result<Vec<TrackResult>> {
        let script_content = fs::read_to_string(script_path)?;

        let provider_table: Table = self.lua.load(&script_content).eval()?;
        let search_func: Function = provider_table.get("search")?;

        // 1. Get the raw Lua Value instead of trying to cast it directly
        let lua_value: mlua::Value = search_func.call_async(query).await?;

        // 2. Explicitly use mlua's Serde integration to parse it into our Rust struct
        let results: Vec<TrackResult> = self.lua.from_value(lua_value)?;

        Ok(results)
    }
}
