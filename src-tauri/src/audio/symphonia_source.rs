use std::{collections::VecDeque, fs::File, path::Path, time::Duration};

use rodio::Source;
use symphonia::core::{
    audio::SampleBuffer,
    codecs::{DecoderOptions, CODEC_TYPE_NULL},
    errors::Error as SymphoniaError,
    formats::{FormatOptions, FormatReader, SeekMode, SeekTo},
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
    units::Time,
};

pub struct SymphoniaSource {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn symphonia::core::codecs::Decoder>,
    track_id: u32,
    sample_rate: u32,
    channels: u16,
    total_duration: Option<Duration>,
    pcm: VecDeque<i16>,
    done: bool,
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

        Ok(Self {
            format,
            decoder,
            track_id,
            sample_rate,
            channels,
            total_duration,
            pcm: VecDeque::with_capacity(8192),
            done: false,
        })
    }

    fn fill(&mut self) -> bool {
        if self.done { return false; }
        loop {
            let packet = match self.format.next_packet() {
                Ok(p) => p,
                Err(_) => {
                    self.done = true;
                    return false;
                }
            };

            if packet.track_id() != self.track_id { continue; }

            match self.decoder.decode(&packet) {
                Ok(decoded) => {
                    // Allocate a fresh interleaved i16 buffer sized for this packet.
                    // copy_interleaved_ref converts from any AudioBufferRef sample type.
                    let mut sbuf = SampleBuffer::<i16>::new(
                        decoded.capacity() as u64,
                        *decoded.spec(),
                    );
                    sbuf.copy_interleaved_ref(decoded);
                    self.pcm.extend(sbuf.samples().iter().copied());
                    return true;
                }
                Err(SymphoniaError::DecodeError(_)) => continue,
                Err(_) => {
                    self.done = true;
                    return false;
                }
            }
        }
    }
}

impl Iterator for SymphoniaSource {
    type Item = i16;

    fn next(&mut self) -> Option<i16> {
        if self.pcm.is_empty() && !self.fill() {
            return None;
        }
        self.pcm.pop_front()
    }
}

impl Source for SymphoniaSource {
    fn current_frame_len(&self) -> Option<usize> { None }
    fn channels(&self) -> u16 { self.channels }
    fn sample_rate(&self) -> u32 { self.sample_rate }
    fn total_duration(&self) -> Option<Duration> { self.total_duration }
}
