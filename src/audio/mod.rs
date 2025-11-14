pub mod player;
pub mod track;
pub mod scanner;
pub mod metadata_parser;
pub mod playlist;

pub use player::{AudioPlayer, PlaybackState};
pub use track::{Track, TrackMetadata};
pub use scanner::MusicScanner;



#[derive(Debug, Clone)]
pub struct AudioConfig {
    pub volume: f32, // 0.0 to 1.0
    pub crossfade_duration: u64, // milliseconds
    pub fade_in_duration: u64, // milliseconds for smooth track start
    pub fade_out_duration: u64, // milliseconds for smooth track stop
    pub buffer_size: usize,
    pub sample_rate: u32,
    pub channels: u16,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            volume: 0.7,
            crossfade_duration: 500,
            fade_in_duration: 300,  // 300ms smooth fade in
            fade_out_duration: 200, // 200ms smooth fade out
            buffer_size: 65536, // Even larger buffer (16x) for ALSA underrun prevention
            sample_rate: 44100, // Standard CD quality
            channels: 2, // Stereo
        }
    }
}

impl From<crate::config::Config> for AudioConfig {
    fn from(_config: crate::config::Config) -> Self {
        // For now, use default audio config
        // Later we can add audio-specific config to the main Config
        AudioConfig::default()
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AudioFormat {
    Mp3,
    Flac,
    Ogg,
    Mp4,
    Wav,
    Unknown,
}

impl AudioFormat {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "mp3" => AudioFormat::Mp3,
            "flac" => AudioFormat::Flac,
            "ogg" | "oga" => AudioFormat::Ogg,
            "mp4" | "m4a" | "aac" => AudioFormat::Mp4,
            "wav" => AudioFormat::Wav,
            _ => AudioFormat::Unknown,
        }
    }
    
    pub fn is_supported(&self) -> bool {
        !matches!(self, AudioFormat::Unknown)
    }
}
