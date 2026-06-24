use rodio::{OutputStream, Sink, Source};
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub mod commands;
mod symphonia_source;

use symphonia_source::SymphoniaSource;

#[derive(serde::Serialize, Clone)]
pub struct PlayerSync {
    pub state: String,
    pub position: f64,
    pub duration: f64,
    pub track: String,
}

pub enum AudioCommand {
    Load(String),
    Play,
    Pause,
    Stop,
    Seek(f64),
    SetVolume(f32),
    SetMute(bool),
    Quit,
}

pub fn start_audio_thread(rx: Receiver<AudioCommand>, app_handle: AppHandle) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let (_stream, stream_handle) =
            OutputStream::try_default().expect("Failed to get audio output");
        let sink = Sink::try_new(&stream_handle).expect("Failed to create audio sink");

        let mut current_track_path = String::new();
        let mut current_duration: f64 = 0.0;
        let mut current_volume: f32 = 1.0;
        let mut is_muted: bool = false;

        let emit_sync = |handle: &AppHandle, state: &str, sink: &Sink, track: &str, duration: f64| {
            let _ = handle.emit("player-sync", PlayerSync {
                state: state.to_string(),
                position: sink.get_pos().as_secs_f64(),
                duration,
                track: track.to_string(),
            });
        };

        loop {
            // Track End Detection
            if sink.empty() && !current_track_path.is_empty() {
                let _ = app_handle.emit("track-ended", ());
                current_track_path.clear();
                current_duration = 0.0;
            }

            match rx.try_recv() {
                Ok(cmd) => {
                    match cmd {
                        AudioCommand::Load(path) => {
                            sink.stop();
                            match SymphoniaSource::from_path(std::path::Path::new(&path)) {
                                Ok(src) => {
                                    current_duration = src.total_duration()
                                        .map(|d| d.as_secs_f64())
                                        .unwrap_or(0.0);
                                    sink.append(src);
                                    sink.play();
                                    current_track_path = path;
                                    emit_sync(&app_handle, "Playing", &sink, &current_track_path, current_duration);
                                }
                                Err(e) => eprintln!("Audio load failed: {e}"),
                            }
                        }
                        AudioCommand::Play => {
                            sink.play();
                            emit_sync(&app_handle, "Playing", &sink, &current_track_path, current_duration);
                        }
                        AudioCommand::Pause => {
                            sink.pause();
                            emit_sync(&app_handle, "Paused", &sink, &current_track_path, current_duration);
                        }
                        AudioCommand::Stop => {
                            sink.stop();
                            current_track_path.clear();
                            current_duration = 0.0;
                            emit_sync(&app_handle, "Stopped", &sink, "", 0.0);
                        }
                        AudioCommand::Seek(pos) => {
                            if current_track_path.is_empty() { continue; }
                            let was_paused = sink.is_paused();
                            let seek_pos = Duration::from_secs_f64(pos);

                            match SymphoniaSource::from_path_seeked(
                                std::path::Path::new(&current_track_path),
                                seek_pos,
                            ) {
                                Ok(src) => {
                                    sink.stop();
                                    sink.append(src);
                                    if !was_paused { sink.play(); }
                                }
                                Err(e) => eprintln!("Audio seek failed: {e}"),
                            }

                            let state_str = if sink.is_paused() { "Paused" } else { "Playing" };
                            let _ = app_handle.emit("player-sync", PlayerSync {
                                state: state_str.to_string(),
                                position: pos,
                                duration: current_duration,
                                track: current_track_path.clone(),
                            });
                        }
                        AudioCommand::SetVolume(vol) => {
                            current_volume = vol;
                            if !is_muted {
                                sink.set_volume(current_volume);
                            }
                        }
                        AudioCommand::SetMute(muted) => {
                            is_muted = muted;
                            if is_muted {
                                sink.set_volume(0.0);
                            } else {
                                sink.set_volume(current_volume);
                            }
                        }
                        AudioCommand::Quit => {
                            break;
                        }
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    thread::sleep(Duration::from_millis(50));
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    break;
                }
            }
        }
    })
}
