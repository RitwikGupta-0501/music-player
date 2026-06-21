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
    Ok(())
}

pub fn insert_tracks(conn: &mut Connection, tracks: Vec<TrackData>) -> Result<usize, String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let mut added = 0;
    
    // Pre-prepare inside the transaction to avoid compiling on every loop
    let mut album_select = tx.prepare("SELECT id FROM albums WHERE title = ?1 AND artist = ?2").map_err(|e| e.to_string())?;

    for track in tracks {
        let album_title = track.album.unwrap_or_else(|| "Unknown Album".to_string());
        let album_artist = track.artist.clone().unwrap_or_else(|| "Unknown Artist".to_string());

        let _ = tx.execute(
            "INSERT OR IGNORE INTO albums (title, artist) VALUES (?1, ?2)",
            (&album_title, &album_artist),
        );
        
        let album_id: Option<i64> = album_select.query_row((&album_title, &album_artist), |row| row.get(0)).ok();

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
