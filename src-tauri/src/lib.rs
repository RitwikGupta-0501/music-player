// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Mutex;
use std::thread;
use tauri::State;

// 1. Define the Message Payload
pub enum AudioCommand {
    Load(String),
    Play,
    Pause,
    Stop,
}

// 2. Define the State structure for Tauri to manage
struct AudioState {
    tx: Mutex<Sender<AudioCommand>>,
}

// 3. The Background Audio Thread
fn start_audio_thread(rx: Receiver<AudioCommand>) {
    thread::spawn(move || {
        // Initialize the audio context. This must live inside the thread.
        let (_stream, stream_handle) =
            OutputStream::try_default().expect("Failed to get audio output device");
        let sink = Sink::try_new(&stream_handle).expect("Failed to create audio sink");

        // The Receiver Loop
        loop {
            // Block and wait for a command from the UI
            if let Ok(cmd) = rx.recv() {
                match cmd {
                    AudioCommand::Load(path) => {
                        sink.stop(); // Clear the current buffer

                        if let Ok(file) = File::open(&path) {
                            let reader = BufReader::new(file);
                            if let Ok(decoder) = Decoder::new(reader) {
                                sink.append(decoder);
                                sink.play(); // Auto-play on load
                            } else {
                                eprintln!("Failed to decode file: {}", path);
                            }
                        } else {
                            eprintln!("Failed to open file: {}", path);
                        }
                    }
                    AudioCommand::Play => sink.play(),
                    AudioCommand::Pause => sink.pause(),
                    AudioCommand::Stop => sink.stop(),
                }
            }
        }
    });
}

// 4. Tauri Commands (The API for the Frontend)
#[tauri::command]
fn load_audio(state: State<'_, AudioState>, path: String) {
    if let Ok(tx) = state.tx.lock() {
        let _ = tx.send(AudioCommand::Load(path));
    }
}

#[tauri::command]
fn play_audio(state: State<'_, AudioState>) {
    if let Ok(tx) = state.tx.lock() {
        let _ = tx.send(AudioCommand::Play);
    }
}

#[tauri::command]
fn pause_audio(state: State<'_, AudioState>) {
    if let Ok(tx) = state.tx.lock() {
        let _ = tx.send(AudioCommand::Pause);
    }
}

#[tauri::command]
fn stop_audio(state: State<'_, AudioState>) {
    if let Ok(tx) = state.tx.lock() {
        let _ = tx.send(AudioCommand::Stop);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Create the channel
    let (tx, rx) = mpsc::channel();

    // Spawn the audio daemon
    start_audio_thread(rx);

    tauri::Builder::default()
        .manage(AudioState { tx: Mutex::new(tx) }) // Inject the Sender into Tauri state
        .invoke_handler(tauri::generate_handler![
            load_audio,
            play_audio,
            pause_audio,
            stop_audio
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
