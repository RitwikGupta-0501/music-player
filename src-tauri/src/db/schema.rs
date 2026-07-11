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
            track_id INTEGER NOT NULL,
            position INTEGER NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(queue_state_id) REFERENCES queue_state(id) ON DELETE CASCADE,
            FOREIGN KEY(track_id) REFERENCES tracks(id) ON DELETE CASCADE
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

    Ok(conn)
}
