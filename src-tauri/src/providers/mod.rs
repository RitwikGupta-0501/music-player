use mlua::{Function, Lua, LuaSerdeExt, Table};
use serde::{Deserialize, Serialize};
use std::fs;

pub mod sandbox;

#[derive(Debug, Serialize, Deserialize)]
pub struct TrackResult {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub stream_url: String,
}

pub struct ProviderManager {
    lua: Lua,
    search_fn: mlua::Function,
}

impl ProviderManager {
    pub fn new(script_path: &std::path::Path) -> mlua::Result<Self> {
        let lua = Lua::new();

        sandbox::configure_sandbox(&lua)?;

        let http_table = lua.create_table()?;

        let get_func = lua.create_async_function(|_lua, url: String| async move {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .map_err(mlua::Error::external)?;
            let res = client.get(&url).send().await.map_err(mlua::Error::external)?;

            let text = res.text().await.map_err(mlua::Error::external)?;

            Ok(text)
        })?;

        http_table.set("get", get_func)?;
        lua.globals().set("http", http_table)?;

        let script_content = fs::read_to_string(script_path).map_err(mlua::Error::external)?;
        let provider_table: Table = lua.load(&script_content).eval()?;
        let search_fn: Function = provider_table.get("search")?;

        Ok(Self { lua, search_fn })
    }

    pub async fn search(&self, query: &str) -> mlua::Result<Vec<TrackResult>> {
        let lua_value: mlua::Value = self.search_fn.call_async(query).await?;
        let results: Vec<TrackResult> = self.lua.from_value(lua_value)?;
        Ok(results)
    }
}
