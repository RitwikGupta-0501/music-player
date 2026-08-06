use rusqlite::Connection;
use std::sync::mpsc::Receiver;
use tokio::sync::oneshot;

pub mod schema;
pub mod queries;

use crate::{Album, LocalTrack, Playlist};

pub struct TrackData {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub track_number: Option<i64>,
    pub file_path: String,
}

pub enum DbRequest {
    GetLocalTracks { limit: u32, offset: u32, resp: oneshot::Sender<Result<Vec<LocalTrack>, String>> },
    GetAlbums { limit: u32, offset: u32, resp: oneshot::Sender<Result<Vec<Album>, String>> },
    GetAlbumTracks { album_id: i64, limit: u32, offset: u32, resp: oneshot::Sender<Result<Vec<LocalTrack>, String>> },
    GetPlaylists { limit: u32, offset: u32, resp: oneshot::Sender<Result<Vec<Playlist>, String>> },
    CreatePlaylist { name: String, resp: oneshot::Sender<Result<i64, String>> },
    AddToPlaylist { playlist_id: i64, track_id: i64, resp: oneshot::Sender<Result<(), String>> },
    GetPlaylistTracks { playlist_id: i64, limit: u32, offset: u32, resp: oneshot::Sender<Result<Vec<LocalTrack>, String>> },
    RemoveFromPlaylist { playlist_id: i64, track_id: i64, resp: oneshot::Sender<Result<(), String>> },
    DeletePlaylist { playlist_id: i64, resp: oneshot::Sender<Result<(), String>> },
    RenamePlaylist { playlist_id: i64, new_name: String, resp: oneshot::Sender<Result<(), String>> },
    ReorderPlaylistTrack { playlist_id: i64, from_pos: i64, to_pos: i64, resp: oneshot::Sender<Result<(), String>> },
    ClearLocalLibrary { resp: oneshot::Sender<Result<(), String>> },
    GetSetting { key: String, resp: oneshot::Sender<Result<Option<String>, String>> },
    SetSetting { key: String, value: String, resp: oneshot::Sender<Result<(), String>> },
    FactoryReset { resp: oneshot::Sender<Result<(), String>> },
    InsertTracks { tracks: Vec<TrackData>, resp: oneshot::Sender<Result<usize, String>> },
    LoadAudioCache { path: String, resp: oneshot::Sender<Result<(), String>> },
    RemoveTrackByPath { path: String, resp: oneshot::Sender<Result<(), String>> },
    SearchLibrary { query: String, limit: u32, resp: oneshot::Sender<Result<Vec<LocalTrack>, String>> },
    SyncProviders { providers: Vec<crate::ProviderInfo>, resp: oneshot::Sender<Result<(), String>> },
    GetProviders { resp: oneshot::Sender<Result<Vec<crate::ProviderInfo>, String>> },
    ToggleProvider { provider_id: String, enabled: bool, resp: oneshot::Sender<Result<(), String>> },
    SaveProviderSettings { provider_id: String, settings_json: String, resp: oneshot::Sender<Result<(), String>> },
    Quit,
}

pub fn start_db_thread(mut conn: Connection, rx: Receiver<DbRequest>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        while let Ok(req) = rx.recv() {
            match req {
                DbRequest::GetLocalTracks { limit, offset, resp } => {
                    let _ = resp.send(queries::get_local_tracks(&conn, limit, offset));
                }
                DbRequest::GetAlbums { limit, offset, resp } => {
                    let _ = resp.send(queries::get_albums(&conn, limit, offset));
                }
                DbRequest::GetAlbumTracks { album_id, limit, offset, resp } => {
                    let _ = resp.send(queries::get_album_tracks(&conn, album_id, limit, offset));
                }
                DbRequest::GetPlaylists { limit, offset, resp } => {
                    let _ = resp.send(queries::get_playlists(&conn, limit, offset));
                }
                DbRequest::CreatePlaylist { name, resp } => {
                    let _ = resp.send(queries::create_playlist(&conn, &name));
                }
                DbRequest::AddToPlaylist { playlist_id, track_id, resp } => {
                    let _ = resp.send(queries::add_to_playlist(&conn, playlist_id, track_id));
                }
                DbRequest::GetPlaylistTracks { playlist_id, limit, offset, resp } => {
                    let _ = resp.send(queries::get_playlist_tracks(&conn, playlist_id, limit, offset));
                }
                DbRequest::RemoveFromPlaylist { playlist_id, track_id, resp } => {
                    let _ = resp.send(queries::remove_from_playlist(&mut conn, playlist_id, track_id));
                }
                DbRequest::DeletePlaylist { playlist_id, resp } => {
                    let _ = resp.send(queries::delete_playlist(&conn, playlist_id));
                }
                DbRequest::RenamePlaylist { playlist_id, new_name, resp } => {
                    let _ = resp.send(queries::rename_playlist(&conn, playlist_id, &new_name));
                }
                DbRequest::ReorderPlaylistTrack { playlist_id, from_pos, to_pos, resp } => {
                    let _ = resp.send(queries::reorder_playlist_track(&mut conn, playlist_id, from_pos, to_pos));
                }
                DbRequest::ClearLocalLibrary { resp } => {
                    let _ = resp.send(queries::clear_local_library(&conn));
                }
                DbRequest::GetSetting { key, resp } => {
                    let _ = resp.send(queries::get_setting(&conn, &key));
                }
                DbRequest::SetSetting { key, value, resp } => {
                    let _ = resp.send(queries::set_setting(&conn, &key, &value));
                }
                DbRequest::FactoryReset { resp } => {
                    let _ = resp.send(queries::factory_reset(&conn));
                }
                DbRequest::InsertTracks { tracks, resp } => {
                    let _ = resp.send(queries::insert_tracks(&mut conn, tracks));
                }
                DbRequest::LoadAudioCache { path, resp } => {
                    let _ = resp.send(queries::load_audio_cache(&conn, &path));
                }
                DbRequest::RemoveTrackByPath { path, resp } => {
                    let _ = resp.send(queries::remove_track_by_path(&conn, &path));
                }
                DbRequest::SearchLibrary { query, limit, resp } => {
                    let _ = resp.send(queries::search_library(&conn, &query, limit));
                }
                DbRequest::SyncProviders { providers, resp } => {
                    let _ = resp.send(queries::sync_providers(&conn, providers));
                }
                DbRequest::GetProviders { resp } => {
                    let _ = resp.send(queries::get_providers(&conn));
                }
                DbRequest::ToggleProvider { provider_id, enabled, resp } => {
                    let _ = resp.send(queries::toggle_provider(&conn, &provider_id, enabled));
                }
                DbRequest::SaveProviderSettings { provider_id, settings_json, resp } => {
                    let _ = resp.send(queries::save_provider_settings(&conn, &provider_id, &settings_json));
                }
                DbRequest::Quit => {
                    break;
                }
            }
        }
    })
}
