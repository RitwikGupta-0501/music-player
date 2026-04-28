-- providers/dummy_search.lua
local provider = {}

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
            stream_url = "/home/ritwik/Music/test_track.mp3"
        }
    }
end

return provider
