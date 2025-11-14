// PanPipe Library - Core modules for terminal music player
// Modular design makes it easy to swap out components

pub mod audio;     // handles playback, scanning, metadata
pub mod behavior;  // tracks what you like/skip
pub mod config;    // settings and preferences
pub mod export;    // playlist export features
pub mod spotify;   // spotify integration (when needed)
pub mod ui;        // terminal interface

// Export the stuff other modules actually use
pub use audio::{AudioPlayer, MusicScanner, Track, TrackMetadata};
pub use behavior::{BehaviorTracker, TrackBehavior, PlaybackEvent, SkipReason};
pub use config::Config;
