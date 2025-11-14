pub mod audio;
pub mod behavior;
pub mod config;
pub mod export;
pub mod spotify;
pub mod ui;

// Re-export commonly used types
pub use audio::{AudioPlayer, MusicScanner, Track, TrackMetadata};
pub use behavior::{BehaviorTracker, TrackBehavior, PlaybackEvent, SkipReason};
pub use config::Config;
