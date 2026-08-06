use rusqlite::Connection;
use crate::{Album, LocalTrack, Playlist};
use super::TrackData;

pub fn get_local_tracks(conn: &Connection, limit: u32, offset: u32) -> Result<Vec<LocalTrack>, String> {
    let mut tracks = Vec::new();
    let mut stmt = conn.prepare("SELECT id, title, artist, album_id, track_number, file_path FROM tracks ORDER BY id LIMIT ?1 OFFSET ?2").map_err(|e| e.to_string())?;
    let track_iter = stmt.query_map([&limit, &offset], |row| {
        Ok(LocalTrack {
            id: row.get(0)?,
            title: row.get(1)?,
            artist: row.get(2)?,
            album_id: row.get(3)?,
            track_number: row.get(4)?,
            file_path: row.get(5)?,
        })
    }).map_err(|e| e.to_string())?;

    for t in track_iter.flatten() {
        tracks.push(t);
    }
    Ok(tracks)
}

pub fn get_albums(conn: &Connection, limit: u32, offset: u32) -> Result<Vec<Album>, String> {
    let mut albums = Vec::new();
    let mut stmt = conn.prepare("SELECT id, title, artist, cover_art_path FROM albums ORDER BY artist, title LIMIT ?1 OFFSET ?2").map_err(|e| e.to_string())?;
    let album_iter = stmt.query_map([&limit, &offset], |row| {
        Ok(Album {
            id: row.get(0)?,
            title: row.get(1)?,
            artist: row.get(2)?,
            cover_art_path: row.get(3)?,
        })
    }).map_err(|e| e.to_string())?;

    for a in album_iter.flatten() {
        albums.push(a);
    }
    Ok(albums)
}

pub fn get_album_tracks(conn: &Connection, album_id: i64, limit: u32, offset: u32) -> Result<Vec<LocalTrack>, String> {
    let mut tracks = Vec::new();
    let mut stmt = conn.prepare("SELECT id, title, artist, album_id, track_number, file_path FROM tracks WHERE album_id = ?1 ORDER BY track_number LIMIT ?2 OFFSET ?3").map_err(|e| e.to_string())?;
    let track_iter = stmt.query_map([&album_id, &(limit as i64), &(offset as i64)], |row| {
        Ok(LocalTrack {
            id: row.get(0)?,
            title: row.get(1)?,
            artist: row.get(2)?,
            album_id: row.get(3)?,
            track_number: row.get(4)?,
            file_path: row.get(5)?,
        })
    }).map_err(|e| e.to_string())?;

    for t in track_iter.flatten() {
        tracks.push(t);
    }
    Ok(tracks)
}

pub fn get_playlists(conn: &Connection, limit: u32, offset: u32) -> Result<Vec<Playlist>, String> {
    let mut playlists = Vec::new();
    let mut stmt = conn.prepare("SELECT id, name FROM playlists ORDER BY created_at LIMIT ?1 OFFSET ?2").map_err(|e| e.to_string())?;
    let playlist_iter = stmt.query_map([&limit, &offset], |row| {
        Ok(Playlist {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    }).map_err(|e| e.to_string())?;

    for p in playlist_iter.flatten() {
        playlists.push(p);
    }
    Ok(playlists)
}

pub fn create_playlist(conn: &Connection, name: &str) -> Result<i64, String> {
    conn.execute("INSERT INTO playlists (name) VALUES (?1)", [&name]).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

pub fn add_to_playlist(conn: &Connection, playlist_id: i64, track_id: i64) -> Result<(), String> {
    let mut stmt = conn.prepare("SELECT COALESCE(MAX(position), 0) FROM playlist_tracks WHERE playlist_id = ?1").map_err(|e| e.to_string())?;
    let max_pos: i64 = stmt.query_row([&playlist_id], |row| row.get(0)).unwrap_or(0);
    
    conn.execute("INSERT INTO playlist_tracks (playlist_id, track_id, position) VALUES (?1, ?2, ?3)", [&playlist_id, &track_id, &(max_pos + 1)]).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_playlist_tracks(conn: &Connection, playlist_id: i64, limit: u32, offset: u32) -> Result<Vec<LocalTrack>, String> {
    let mut tracks = Vec::new();
    let mut stmt = conn.prepare("SELECT t.id, t.title, t.artist, t.album_id, t.track_number, t.file_path FROM tracks t JOIN playlist_tracks pt ON t.id = pt.track_id WHERE pt.playlist_id = ?1 ORDER BY pt.position LIMIT ?2 OFFSET ?3").map_err(|e| e.to_string())?;
    let track_iter = stmt.query_map([&playlist_id, &(limit as i64), &(offset as i64)], |row| {
        Ok(LocalTrack {
            id: row.get(0)?,
            title: row.get(1)?,
            artist: row.get(2)?,
            album_id: row.get(3)?,
            track_number: row.get(4)?,
            file_path: row.get(5)?,
        })
    }).map_err(|e| e.to_string())?;

    for t in track_iter.flatten() {
        tracks.push(t);
    }
    Ok(tracks)
}

pub fn remove_from_playlist(conn: &mut Connection, playlist_id: i64, track_id: i64) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    
    let position: Option<i64> = tx.query_row(
        "SELECT position FROM playlist_tracks WHERE playlist_id = ?1 AND track_id = ?2",
        [&playlist_id, &track_id],
        |row| row.get(0)
    ).ok();

    if let Some(pos) = position {
        tx.execute(
            "DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND track_id = ?2",
            [&playlist_id, &track_id]
        ).map_err(|e| e.to_string())?;

        tx.execute(
            "UPDATE playlist_tracks SET position = position - 1 WHERE playlist_id = ?1 AND position > ?2",
            [&playlist_id, &pos]
        ).map_err(|e| e.to_string())?;
    }
    
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_playlist(conn: &Connection, playlist_id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM playlists WHERE id = ?1", [&playlist_id]).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn rename_playlist(conn: &Connection, playlist_id: i64, new_name: &str) -> Result<(), String> {
    conn.execute("UPDATE playlists SET name = ?1 WHERE id = ?2", (new_name, &playlist_id)).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn reorder_playlist_track(conn: &mut Connection, playlist_id: i64, from_pos: i64, to_pos: i64) -> Result<(), String> {
    if from_pos == to_pos {
        return Ok(());
    }

    let tx = conn.transaction().map_err(|e| e.to_string())?;

    tx.execute(
        "UPDATE playlist_tracks SET position = -1 WHERE playlist_id = ?1 AND position = ?2",
        [&playlist_id, &from_pos]
    ).map_err(|e| e.to_string())?;

    if from_pos < to_pos {
        tx.execute(
            "UPDATE playlist_tracks SET position = position - 1 WHERE playlist_id = ?1 AND position > ?2 AND position <= ?3",
            [&playlist_id, &from_pos, &to_pos]
        ).map_err(|e| e.to_string())?;
    } else {
        tx.execute(
            "UPDATE playlist_tracks SET position = position + 1 WHERE playlist_id = ?1 AND position >= ?2 AND position < ?3",
            [&playlist_id, &to_pos, &from_pos]
        ).map_err(|e| e.to_string())?;
    }

    tx.execute(
        "UPDATE playlist_tracks SET position = ?1 WHERE playlist_id = ?2 AND position = -1",
        [&to_pos, &playlist_id]
    ).map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn clear_local_library(conn: &Connection) -> Result<(), String> {
    conn.execute("DELETE FROM tracks", []).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM albums", []).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM playlists", []).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM playlist_tracks", []).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1").map_err(|e| e.to_string())?;
    let mut rows = stmt.query([key]).map_err(|e| e.to_string())?;
    if let Ok(Some(row)) = rows.next() {
        let value: String = row.get(0).map_err(|e| e.to_string())?;
        return Ok(Some(value));
    }
    Ok(None)
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2",
        (key, value),
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn factory_reset(conn: &Connection) -> Result<(), String> {
    let _ = conn.execute("DELETE FROM tracks", []);
    let _ = conn.execute("DELETE FROM settings", []);
    let _ = conn.execute("DELETE FROM albums", []);
    let _ = conn.execute("DELETE FROM playlists", []);
    let _ = conn.execute("DELETE FROM playlist_tracks", []);
    Ok(())
}

pub fn insert_tracks(conn: &mut Connection, tracks: Vec<TrackData>) -> Result<usize, String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let mut added = 0;
    
    // Pre-prepare inside the transaction to avoid compiling on every loop
    let mut album_select = tx.prepare("SELECT id FROM albums WHERE title = ?1 AND artist = ?2").map_err(|e| e.to_string())?;
    let mut album_cache: std::collections::HashMap<(String, String), Option<i64>> = std::collections::HashMap::new();

    for track in tracks {
        let album_title = track.album.unwrap_or_else(|| "Unknown Album".to_string());
        let album_artist = track.artist.clone().unwrap_or_else(|| "Unknown Artist".to_string());
        let cache_key = (album_title.clone(), album_artist.clone());

        let album_id = if let Some(&id) = album_cache.get(&cache_key) {
            id
        } else {
            let _ = tx.execute(
                "INSERT OR IGNORE INTO albums (title, artist) VALUES (?1, ?2)",
                (&album_title, &album_artist),
            );
            let id: Option<i64> = album_select.query_row((&album_title, &album_artist), |row| row.get(0)).ok();
            album_cache.insert(cache_key, id);
            id
        };

        let res = tx.execute(
            "INSERT OR IGNORE INTO tracks (title, artist, album_id, track_number, file_path) VALUES (?1, ?2, ?3, ?4, ?5)",
            (&track.title, &track.artist, &album_id, &track.track_number, &track.file_path),
        );
        
        if res.is_ok() && res.unwrap() > 0 {
            added += 1;
        }
    }
    drop(album_select);
    tx.commit().map_err(|e| e.to_string())?;
    Ok(added)
}

pub fn load_audio_cache(conn: &Connection, path: &str) -> Result<(), String> {
    let _ = conn.execute(
        "INSERT OR IGNORE INTO tracks (title, file_path) VALUES (?1, ?2)",
        ("Unknown Title", path),
    );
    Ok(())
}

pub fn remove_track_by_path(conn: &Connection, path: &str) -> Result<(), String> {
    let album_id: Option<i64> = conn
        .query_row("SELECT album_id FROM tracks WHERE file_path = ?1", [path], |r| r.get(0))
        .ok()
        .flatten();

    conn.execute("DELETE FROM tracks WHERE file_path = ?1", [path])
        .map_err(|e| e.to_string())?;

    // Remove orphaned album if this was its last track.
    if let Some(album_id) = album_id {
        let remaining: i64 = conn
            .query_row("SELECT COUNT(*) FROM tracks WHERE album_id = ?1", [album_id], |r| r.get(0))
            .unwrap_or(0);
        if remaining == 0 {
            let _ = conn.execute("DELETE FROM albums WHERE id = ?1", [album_id]);
        }
    }

    Ok(())
}

pub fn search_library(conn: &Connection, query: &str, limit: u32) -> Result<Vec<LocalTrack>, String> {
    let mut tracks = Vec::new();
    let like_query = format!("%{}%", query);
    
    let mut stmt = conn.prepare(
        "SELECT t.id, t.title, t.artist, t.album_id, t.track_number, t.file_path 
         FROM tracks t 
         LEFT JOIN albums a ON t.album_id = a.id
         WHERE t.title LIKE ?1 OR t.artist LIKE ?1 OR a.title LIKE ?1
         ORDER BY t.title 
         LIMIT ?2"
    ).map_err(|e| e.to_string())?;

    let track_iter = stmt.query_map(rusqlite::params![&like_query, &limit], |row| {
        Ok(LocalTrack {
            id: row.get(0)?,
            title: row.get(1)?,
            artist: row.get(2)?,
            album_id: row.get(3)?,
            track_number: row.get(4)?,
            file_path: row.get(5)?,
        })
    }).map_err(|e| e.to_string())?;

    for t in track_iter.flatten() {
        tracks.push(t);
    }
    Ok(tracks)
}

pub fn sync_providers(conn: &Connection, providers: Vec<crate::ProviderInfo>) -> Result<(), String> {
    for p in providers {
        conn.execute(
            "INSERT INTO providers (id, name, author, version, file_path, status, error_message, checksum, capabilities, homepage, settings_schema, priority, icon)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(id) DO UPDATE SET
                name=excluded.name,
                author=excluded.author,
                version=excluded.version,
                file_path=excluded.file_path,
                error_message=excluded.error_message,
                checksum=excluded.checksum,
                capabilities=excluded.capabilities,
                homepage=excluded.homepage,
                settings_schema=excluded.settings_schema,
                priority=excluded.priority,
                icon=excluded.icon,
                updated_at=CURRENT_TIMESTAMP",
            rusqlite::params![
                p.id, p.name, p.author, p.version, p.file_path,
                p.status, p.error_message, p.checksum,
                p.capabilities.map(|c| serde_json::to_string(&c).unwrap_or_default()),
                p.homepage, p.settings_schema, p.priority, p.icon
            ],
        ).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn get_providers(conn: &Connection) -> Result<Vec<crate::ProviderInfo>, String> {
    let mut stmt = conn.prepare("SELECT id, name, author, version, file_path, status, error_message, checksum, capabilities, homepage, settings_schema, priority, icon, settings FROM providers").map_err(|e| e.to_string())?;
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;

    let mut providers = Vec::new();
    while let Ok(Some(row)) = rows.next() {
        let id: String = row.get(0).unwrap_or_default();
        let capabilities_str: Option<String> = row.get(8).unwrap_or(None);
        let capabilities: Option<Vec<String>> = capabilities_str.and_then(|s| serde_json::from_str(&s).ok());
        
        providers.push(crate::ProviderInfo {
            id: id,
            name: row.get(1).unwrap_or_default(),
            author: row.get(2).unwrap_or_default(),
            version: row.get(3).unwrap_or_default(),
            file_path: row.get(4).unwrap_or_default(),
            status: row.get(5).unwrap_or_else(|_| "enabled".to_string()),
            error_message: row.get(6).unwrap_or(None),
            checksum: row.get(7).unwrap_or(None),
            capabilities,
            homepage: row.get(9).unwrap_or(None),
            settings_schema: row.get(10).unwrap_or(None),
            priority: row.get(11).unwrap_or(0),
            icon: row.get(12).unwrap_or(None),
            settings: row.get(13).unwrap_or(None),
        });
    }

    Ok(providers)
}

pub fn toggle_provider(conn: &Connection, provider_id: &str, enabled: bool) -> Result<(), String> {
    let status = if enabled { "enabled" } else { "disabled" };
    conn.execute(
        "UPDATE providers SET status = ?1 WHERE id = ?2",
        rusqlite::params![status, provider_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn save_provider_settings(conn: &Connection, provider_id: &str, settings_json: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE providers SET settings = ?1 WHERE id = ?2",
        rusqlite::params![settings_json, provider_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}
