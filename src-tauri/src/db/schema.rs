use rusqlite::{Connection, Result as SqlResult};

pub fn init_db<P: AsRef<std::path::Path>>(db_path: P) -> SqlResult<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute("PRAGMA foreign_keys = ON", [])?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS albums (
            id INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            artist TEXT,
            cover_art_path TEXT,
            UNIQUE(title, artist)
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tracks (
            id INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            artist TEXT,
            album_id INTEGER,
            track_number INTEGER,
            file_path TEXT UNIQUE NOT NULL,
            FOREIGN KEY(album_id) REFERENCES albums(id)
        )",
        [],
    )?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_tracks_album_id ON tracks(album_id)", [])?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS playlists (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS playlist_tracks (
            id INTEGER PRIMARY KEY,
            playlist_id INTEGER,
            track_id INTEGER,
            position INTEGER,
            FOREIGN KEY(playlist_id) REFERENCES playlists(id) ON DELETE CASCADE,
            FOREIGN KEY(track_id) REFERENCES tracks(id) ON DELETE CASCADE
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS providers (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            author TEXT NOT NULL,
            version TEXT NOT NULL,
            file_path TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'enabled',
            error_message TEXT,
            checksum TEXT,
            capabilities TEXT,
            homepage TEXT,
            settings_schema TEXT,
            priority INTEGER NOT NULL DEFAULT 0,
            icon TEXT,
            settings TEXT,
            imported_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // Migration to add settings column if it doesn't exist
    let _ = conn.execute("ALTER TABLE providers ADD COLUMN settings TEXT", []);

    // Queue persistence tables (Phase 5)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS queue_state (
            id INTEGER PRIMARY KEY,
            current_position INTEGER NOT NULL DEFAULT 0,
            repeat_mode TEXT NOT NULL DEFAULT 'Off',
            queue_mode TEXT NOT NULL DEFAULT 'Normal',
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS queued_tracks (
            id INTEGER PRIMARY KEY,
            queue_state_id INTEGER NOT NULL,
            instance_id TEXT NOT NULL UNIQUE,
            track_id INTEGER NOT NULL DEFAULT -1,
            position INTEGER NOT NULL,
            -- Remote source fields (NULL for local tracks)
            stream_url TEXT,
            provider_id TEXT,
            remote_track_id TEXT,
            quality_hint TEXT,
            cached_title TEXT,
            cached_artist TEXT,
            cover_art_url TEXT,
            duration_ms INTEGER,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(queue_state_id) REFERENCES queue_state(id) ON DELETE CASCADE
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS shuffle_order (
            id INTEGER PRIMARY KEY,
            queue_state_id INTEGER NOT NULL,
            instance_id TEXT NOT NULL,
            position INTEGER NOT NULL,
            seed INTEGER NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(queue_state_id) REFERENCES queue_state(id) ON DELETE CASCADE
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS queue_history (
            id INTEGER PRIMARY KEY,
            action TEXT NOT NULL,
            from_position INTEGER,
            to_position INTEGER,
            track_id INTEGER,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS queue_snapshots (
            id INTEGER PRIMARY KEY,
            queue_state_id INTEGER NOT NULL,
            description TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(queue_state_id) REFERENCES queue_state(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Migrations for existing DBs
    let _ = conn.execute("ALTER TABLE queued_tracks ADD COLUMN remote_track_id TEXT", []);
    let _ = conn.execute("ALTER TABLE queued_tracks ADD COLUMN duration_ms INTEGER", []);
    
    let _ = conn.execute("DROP TABLE IF EXISTS provider_states", []);
    let _ = conn.execute(
        "CREATE TABLE IF NOT EXISTS providers (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            author TEXT NOT NULL,
            version TEXT NOT NULL,
            file_path TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'enabled',
            error_message TEXT,
            checksum TEXT,
            capabilities TEXT,
            homepage TEXT,
            settings_schema TEXT,
            priority INTEGER NOT NULL DEFAULT 0,
            icon TEXT,
            imported_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    );

    Ok(conn)
}
