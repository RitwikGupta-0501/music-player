use rodio::{Decoder, OutputStream, Sink};
use rusqlite::{Connection, Result as SqlResult};
use std::fs::File;
use std::io::BufReader;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Mutex;
use std::thread;
use tauri::{AppHandle, Emitter, State};
mod providers;
use providers::{ProviderManager, TrackResult};

// 1. Message Payload
pub enum AudioCommand {
    Load(String),
    Play,
    Pause,
    Stop,
}

// 2. Global State
struct AppState {
    tx: Mutex<Sender<AudioCommand>>,
    db_conn: Mutex<Connection>,
}

// 3. Database Initialization
fn init_db() -> SqlResult<Connection> {
    // Creates a local file for now.
    let conn = Connection::open("echo_library.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tracks (
            id INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            file_path TEXT UNIQUE NOT NULL
        )",
        [],
    )?;
    Ok(conn)
}

// 4. The Audio Thread (Now with Event Emission)
fn start_audio_thread(rx: Receiver<AudioCommand>, app_handle: AppHandle) {
    thread::spawn(move || {
        let (_stream, stream_handle) =
            OutputStream::try_default().expect("Failed to get audio output");
        let sink = Sink::try_new(&stream_handle).expect("Failed to create audio sink");

        loop {
            if let Ok(cmd) = rx.recv() {
                match cmd {
                    AudioCommand::Load(path) => {
                        sink.stop();
                        if let Ok(file) = File::open(&path) {
                            let reader = BufReader::new(file);
                            if let Ok(decoder) = Decoder::new(reader) {
                                sink.append(decoder);
                                sink.play();
                                // Emit state back to Svelte
                                let _ = app_handle.emit("player-state", "Playing");
                                let _ = app_handle.emit("current-track", path.clone());
                            }
                        }
                    }
                    AudioCommand::Play => {
                        sink.play();
                        let _ = app_handle.emit("player-state", "Playing");
                    }
                    AudioCommand::Pause => {
                        sink.pause();
                        let _ = app_handle.emit("player-state", "Paused");
                    }
                    AudioCommand::Stop => {
                        sink.stop();
                        let _ = app_handle.emit("player-state", "Stopped");
                    }
                }
            }
        }
    });
}

#[tauri::command]
async fn search_provider(query: String) -> Result<Vec<TrackResult>, String> {
    // In a production app, we would load the manager once into Tauri state,
    // but for testing the bridge, we spin it up on demand.
    let manager = ProviderManager::new().map_err(|e| e.to_string())?;

    // We point this to our local test script
    let results = manager
        .search("../providers/dummy_search.lua", &query)
        .await
        .map_err(|e| e.to_string())?;

    Ok(results)
}

// 5. Async Tauri Commands
#[tauri::command]
async fn load_audio(state: State<'_, AppState>, path: String) -> Result<(), String> {
    // Cache the track in SQLite when loaded
    if let Ok(conn) = state.db_conn.lock() {
        let _ = conn.execute(
            "INSERT OR IGNORE INTO tracks (title, file_path) VALUES (?1, ?2)",
            ("Unknown Title", &path),
        );
    }

    if let Ok(tx) = state.tx.lock() {
        let _ = tx.send(AudioCommand::Load(path));
    }
    Ok(())
}

#[tauri::command]
async fn play_audio(state: State<'_, AppState>) -> Result<(), String> {
    if let Ok(tx) = state.tx.lock() {
        let _ = tx.send(AudioCommand::Play);
    }
    Ok(())
}

#[tauri::command]
async fn pause_audio(state: State<'_, AppState>) -> Result<(), String> {
    if let Ok(tx) = state.tx.lock() {
        let _ = tx.send(AudioCommand::Pause);
    }
    Ok(())
}

#[tauri::command]
async fn stop_audio(state: State<'_, AppState>) -> Result<(), String> {
    if let Ok(tx) = state.tx.lock() {
        let _ = tx.send(AudioCommand::Stop);
    }
    Ok(())
}

// 6. App Initialization
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let (tx, rx) = mpsc::channel();
    let db = init_db().expect("Failed to initialize SQLite");

    tauri::Builder::default()
        .setup(|app| {
            // Pass a clone of the AppHandle into the audio thread
            let handle = app.handle().clone();
            start_audio_thread(rx, handle);
            Ok(())
        })
        .manage(AppState {
            tx: Mutex::new(tx),
            db_conn: Mutex::new(db),
        })
        .invoke_handler(tauri::generate_handler![
            load_audio,
            play_audio,
            pause_audio,
            stop_audio,
            search_provider
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
