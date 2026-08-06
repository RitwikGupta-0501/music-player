use std::{fs::File, path::Path, time::Duration, io::{Read, Seek, SeekFrom}};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

// ── StreamMediaSource Wrapper ───────────────────────────────────────────────

pub struct StreamMediaSource<T> {
    inner: T,
    content_length: Option<u64>,
}

impl<T> StreamMediaSource<T> {
    pub fn new(inner: T, content_length: Option<u64>) -> Self {
        Self { inner, content_length }
    }
}

impl<T: Read> Read for StreamMediaSource<T> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

impl<T: Seek> Seek for StreamMediaSource<T> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

impl<T: Read + Seek + Send + Sync> symphonia::core::io::MediaSource for StreamMediaSource<T> {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        self.content_length
    }
}

use ringbuf::{Consumer, HeapRb};
use rodio::Source;
use symphonia::core::{
    audio::SampleBuffer,
    codecs::{DecoderOptions, CODEC_TYPE_NULL},
    errors::Error as SymphoniaError,
    formats::{FormatOptions, SeekMode, SeekTo},
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
    units::Time,
};
use stream_download::{StreamDownload, Settings, http::HttpStream};

pub struct SymphoniaSource {
    consumer: Consumer<i16, Arc<HeapRb<i16>>>,
    channels: u16,
    sample_rate: u32,
    total_duration: Option<Duration>,
    abort_flag: Arc<AtomicBool>,
    eof_flag: Arc<AtomicBool>,
}

impl SymphoniaSource {
    pub fn from_path(path: &Path) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::open(Box::new(File::open(path)?), path.extension().and_then(|e| e.to_str()), None)
    }

    pub fn from_path_seeked(
        path: &Path,
        pos: Duration,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::open(Box::new(File::open(path)?), path.extension().and_then(|e| e.to_str()), Some(pos))
    }

    pub fn from_url(
        url: url::Url,
        client: reqwest::Client,
        runtime_handle: tauri::async_runtime::RuntimeHandle,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::from_url_internal(url, client, runtime_handle, None)
    }

    pub fn from_url_seeked(
        url: url::Url,
        client: reqwest::Client,
        runtime_handle: tauri::async_runtime::RuntimeHandle,
        seek_to: Duration,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::from_url_internal(url, client, runtime_handle, Some(seek_to))
    }

    pub fn from_hls(
        url: url::Url,
        client: reqwest::Client,
        runtime_handle: tauri::async_runtime::RuntimeHandle,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let stream = crate::audio::hls::HlsStream::new(url, client, runtime_handle, None)?;
        Self::open(Box::new(stream), None, None)
    }

    pub fn from_hls_seeked(
        url: url::Url,
        client: reqwest::Client,
        runtime_handle: tauri::async_runtime::RuntimeHandle,
        seek_to: Duration,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // HLS seeking is handled by skipping segments internally during instantiation
        let stream = crate::audio::hls::HlsStream::new(url, client, runtime_handle, Some(seek_to))?;
        // Pass seek_to = None to `open` so Symphonia doesn't try to byte-seek
        Self::open(Box::new(stream), None, None)
    }

    fn from_url_internal(
        url: url::Url,
        client: reqwest::Client,
        runtime_handle: tauri::async_runtime::RuntimeHandle,
        seek_to: Option<Duration>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let (download, content_length) = runtime_handle.block_on(async {
            use stream_download::source::SourceStream;
            let stream = HttpStream::new(client, url.clone()).await?;
            let content_length = stream.content_length();
            let settings = Settings::default().prefetch_bytes(5 * 1024 * 1024); // 5MB buffer
            
            // Use TempStorageProvider instead of BoundedStorageProvider to avoid subtraction overflow
            // panics when the MP4 demuxer seeks backward from the end of the file.
            let storage = stream_download::storage::temp::TempStorageProvider::new();
            
            let download = StreamDownload::from_stream(stream, storage, settings).await?;
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>((download, content_length))
        })?;
        let wrapper = StreamMediaSource::new(download, content_length); // Pass the fetched content length
        Self::open(Box::new(wrapper), None, seek_to)
    }

    fn open(
        media_source: Box<dyn symphonia::core::io::MediaSource>,
        extension: Option<&str>,
        seek_to: Option<Duration>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mss = MediaSourceStream::new(media_source, Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = extension {
            hint.with_extension(ext);
        }

        let probed = symphonia::default::get_probe().format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )?;

        let mut format = probed.format;

        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or("no supported audio track")?;

        let track_id = track.id;
        let codec_params = track.codec_params.clone();

        let sample_rate = codec_params.sample_rate.unwrap_or(44100);
        let channels = codec_params.channels.map(|c| c.count() as u16).unwrap_or(2);

        let total_duration = codec_params
            .n_frames
            .zip(codec_params.sample_rate)
            .map(|(frames, rate)| Duration::from_secs_f64(frames as f64 / rate as f64));

        let mut decoder = symphonia::default::get_codecs()
            .make(&codec_params, &DecoderOptions::default())?;

        if let Some(pos) = seek_to {
            let secs = pos.as_secs_f64();
            // SeekMode::Coarse works even for FLAC files without a SEEKTABLE —
            // Symphonia falls back to a bisection search over the byte stream.
            let _ = format.seek(
                SeekMode::Coarse,
                SeekTo::Time {
                    time: Time { seconds: secs as u64, frac: secs.fract() },
                    track_id: None,
                },
            );
            decoder.reset();
        }

        // Create exactly ~4 seconds of buffer based on sample rate and channels
        let buffer_capacity = sample_rate as usize * channels as usize * 4;
        let rb = HeapRb::<i16>::new(buffer_capacity);
        let (mut producer, consumer) = rb.split();

        let abort_flag = Arc::new(AtomicBool::new(false));
        let eof_flag = Arc::new(AtomicBool::new(false));

        let abort_clone = abort_flag.clone();
        let eof_clone = eof_flag.clone();

        // Spawn background decoder thread
        thread::spawn(move || {
            loop {
                // Check if audio thread dropped us
                if abort_clone.load(Ordering::Acquire) {
                    break;
                }

                // If buffer is full, yield CPU to wait for consumer
                if producer.is_full() {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }

                let packet = match format.next_packet() {
                    Ok(p) => p,
                    Err(_) => {
                        eof_clone.store(true, Ordering::Release);
                        break;
                    }
                };

                if packet.track_id() != track_id {
                    continue;
                }

                match decoder.decode(&packet) {
                    Ok(decoded) => {
                        let mut sbuf = SampleBuffer::<i16>::new(
                            decoded.capacity() as u64,
                            *decoded.spec(),
                        );
                        sbuf.copy_interleaved_ref(decoded);
                        
                        let samples = sbuf.samples();
                        let mut pushed = 0;
                        
                        // Push samples to the ring buffer. This might take a few iterations if the 
                        // buffer gets full mid-packet.
                        while pushed < samples.len() {
                            if abort_clone.load(Ordering::Acquire) {
                                return;
                            }
                            
                            let pushed_now = producer.push_slice(&samples[pushed..]);
                            pushed += pushed_now;
                            
                            if pushed < samples.len() {
                                thread::sleep(Duration::from_millis(5));
                            }
                        }
                    }
                    Err(SymphoniaError::DecodeError(_)) => continue,
                    Err(_) => {
                        eof_clone.store(true, Ordering::Release);
                        break;
                    }
                }
            }
        });

        Ok(Self {
            consumer,
            channels,
            sample_rate,
            total_duration,
            abort_flag,
            eof_flag,
        })
    }
}

impl Iterator for SymphoniaSource {
    type Item = i16;

    fn next(&mut self) -> Option<i16> {
        match self.consumer.pop() {
            Some(sample) => Some(sample),
            None => {
                if self.eof_flag.load(Ordering::Acquire) {
                    None // Track actually ended
                } else {
                    Some(0) // Underrun masked with silence; never blocks!
                }
            }
        }
    }
}

impl Source for SymphoniaSource {
    fn current_frame_len(&self) -> Option<usize> { None }
    fn channels(&self) -> u16 { self.channels }
    fn sample_rate(&self) -> u32 { self.sample_rate }
    fn total_duration(&self) -> Option<Duration> { self.total_duration }
}

impl Drop for SymphoniaSource {
    fn drop(&mut self) {
        self.abort_flag.store(true, Ordering::Release);
    }
}
