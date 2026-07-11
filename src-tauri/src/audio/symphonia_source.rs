use std::{fs::File, path::Path, time::Duration};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

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
        Self::open(path, None)
    }

    pub fn from_path_seeked(
        path: &Path,
        pos: Duration,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::open(path, Some(pos))
    }

    fn open(
        path: &Path,
        seek_to: Option<Duration>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let file = File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
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
