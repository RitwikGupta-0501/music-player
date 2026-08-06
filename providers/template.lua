local provider = {}

provider.metadata = {
    name = "Ultimate Stream API",
    author = "Echo Community",
    version = "1.0.0",
    features = { "search", "stream", "radio", "lyrics" }
}

-- 1. Discovery
function provider.search(query, type)
    -- type could be "track", "album", or "artist"
    -- Returns: { { id, title, artist, album, cover_art_url, stream_url }, ... }
end

-- 2. Playback Resolution (Called right before playback)
function provider.get_stream(track_id)
    -- Returns: { url = "https://...", format = "mp3", bitrate = 320 }
end

-- 3. The Infinite Queue (The Algorithmic Radio)
function provider.get_recommendations(seed_track_id)
    -- Returns an array of tracks similar to the seed
    -- Returns: { { id, title, artist, album, cover_art_url, stream_url }, ... }
end

-- 4. Experience
function provider.get_lyrics(track_id)
    -- Returns: { synced = true, content = "[00:15.00]First line of the song\n..." }
end

return provider
