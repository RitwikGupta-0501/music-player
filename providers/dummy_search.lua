-- providers/dummy_search.lua
local provider = {}

provider.metadata = {
    id = "dummy_search",
    name = "Dummy Search",
    author = "Echo Local",
    version = "1.0.0",
    homepage = "https://example.com",
    capabilities = {"search"},
    settings_schema = "{}",
    priority = 1,
    icon = "Ghost"
}

function provider.search(query)
    -- This calls the Rust reqwest client!
    local response_body = http.get("https://dummyjson.com/products/search?q=" .. query)

    -- In a real scenario, we'd parse this JSON. For now, we'll just return
    -- a mock track to prove the pipeline works without crashing.
    return {
        {
            id = "test_1",
            title = "API Hit Success: " .. query,
            artist = "Lua Bridge",
            album = "The Sandbox EP",
            cover_art_url = "https://images.unsplash.com/photo-1614613535308-eb5fbd3d2c17?auto=format&fit=crop&q=80&w=200",
            stream_url = "/home/ritwik/Music/test_track.mp3"
        }
    }
end

return provider
