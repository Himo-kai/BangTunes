use super::{AudioFormat, Track, TrackMetadata};
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use walkdir::WalkDir;

#[derive(Clone)]
pub struct MusicScanner {
    supported_extensions: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum ScanProgress {
    Started { total_directories: usize },
    DirectoryStarted { path: PathBuf },
    TrackFound { track: Track, progress: usize, total: Option<usize> },
    DirectoryCompleted { path: PathBuf, tracks_found: usize },
    Completed { total_tracks: usize },
    Error { path: PathBuf, error: String },
}

impl MusicScanner {
    pub fn new() -> Self {
        Self {
            supported_extensions: vec![
                "mp3".to_string(),
                "flac".to_string(),
                "ogg".to_string(),
                "oga".to_string(),
                "mp4".to_string(),
                "m4a".to_string(),
                "aac".to_string(),
                "wav".to_string(),
            ],
        }
    }

    pub fn scan_directory<P: AsRef<Path>>(&self, path: P) -> Result<Vec<Track>> {
        let mut tracks = Vec::new();
        
        for entry in WalkDir::new(path).follow_links(true).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            
            if entry.file_type().is_file() {
                // Skip hidden files (dotfiles)
                if path.file_name()
                    .and_then(|n| n.to_str())
                    .map_or(false, |n| n.starts_with('.')) {
                    continue;
                }
                
                // Check file size to skip absurd files
                if let Ok(metadata) = fs::metadata(path) {
                    if metadata.len() == 0 || metadata.len() > 1_000_000_000 {
                        // Skip empty files or files > 1GB
                        continue;
                    }
                }
                
                if self.is_supported_file(path) {
                    if let Ok(track) = self.create_track_from_file(path) {
                        tracks.push(track);
                    }
                }
            }
        }
        
        Ok(tracks)
    }

    pub fn scan_directories(&self, paths: &[PathBuf]) -> Result<Vec<Track>> {
        let mut all_tracks = Vec::new();
        
        for path in paths {
            if path.exists() {
                let mut tracks = self.scan_directory(path)?;
                all_tracks.append(&mut tracks);
            }
        }
        
        Ok(all_tracks)
    }

    /// Incremental scan with progress updates via channel for non-blocking UI updates
    pub async fn scan_directories_incremental(
        &self,
        paths: &[PathBuf],
        progress_tx: mpsc::Sender<ScanProgress>,
    ) -> Result<Vec<Track>> {
        let mut all_tracks = Vec::new();
        let total_directories = paths.len();
        
        // Send initial progress
        let _ = progress_tx.send(ScanProgress::Started { total_directories }).await;
        
        for path in paths {
            if !path.exists() {
                let _ = progress_tx.send(ScanProgress::Error {
                    path: path.clone(),
                    error: "Directory does not exist".to_string(),
                }).await;
                continue;
            }
            
            // Send directory start progress
            let _ = progress_tx.send(ScanProgress::DirectoryStarted { path: path.clone() }).await;
            
            let mut directory_tracks = 0;
            let mut progress_count = all_tracks.len();
            
            for entry in WalkDir::new(path).follow_links(true).into_iter().filter_map(Result::ok) {
                let entry_path = entry.path();
                
                if entry.file_type().is_file() {
                    // Skip hidden files (dotfiles)
                    if entry_path.file_name()
                        .and_then(|n| n.to_str())
                        .map_or(false, |n| n.starts_with('.')) {
                        continue;
                    }
                    
                    // Check file size to skip absurd files
                    if let Ok(metadata) = fs::metadata(entry_path) {
                        if metadata.len() == 0 || metadata.len() > 1_000_000_000 {
                            continue;
                        }
                    }
                    
                    if self.is_supported_file(entry_path) {
                        match self.create_track_from_file(entry_path) {
                            Ok(track) => {
                                progress_count += 1;
                                directory_tracks += 1;
                                
                                // Send track found progress
                                let _ = progress_tx.send(ScanProgress::TrackFound {
                                    track: track.clone(),
                                    progress: progress_count,
                                    total: None, // We don't know total until complete
                                }).await;
                                
                                all_tracks.push(track);
                                
                                // Yield control periodically for UI responsiveness
                                if progress_count % 10 == 0 {
                                    tokio::task::yield_now().await;
                                }
                            }
                            Err(e) => {
                                let _ = progress_tx.send(ScanProgress::Error {
                                    path: entry_path.to_path_buf(),
                                    error: e.to_string(),
                                }).await;
                            }
                        }
                    }
                }
            }
            
            // Send directory completion progress
            let _ = progress_tx.send(ScanProgress::DirectoryCompleted {
                path: path.clone(),
                tracks_found: directory_tracks,
            }).await;
        }
        
        // Send final completion progress
        let _ = progress_tx.send(ScanProgress::Completed {
            total_tracks: all_tracks.len(),
        }).await;
        
        Ok(all_tracks)
    }

    fn is_supported_file(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                let normalized = ext.to_ascii_lowercase();
                self.supported_extensions.contains(&normalized)
            })
            .unwrap_or(false)
    }

    fn create_track_from_file(&self, path: &Path) -> Result<Track> {
        let metadata = fs::metadata(path)?;
        let file_size = metadata.len();
        
        let format = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(AudioFormat::from_extension)
            .unwrap_or(AudioFormat::Unknown);

        let mut track = Track::new(path.to_path_buf());
        track.file_size = file_size;
        track.format = format;

        // Extract metadata based on file type
        match &track.format {
            AudioFormat::Mp3 => {
                if let Ok(tag_metadata) = self.extract_id3_metadata(path) {
                    track = track.with_metadata(tag_metadata);
                }
            }
            AudioFormat::Mp4 => {
                if let Ok(tag_metadata) = self.extract_mp4_metadata(path) {
                    track = track.with_metadata(tag_metadata);
                }
            }
            AudioFormat::Flac => {
                if let Ok(tag_metadata) = self.extract_flac_metadata(path) {
                    track = track.with_metadata(tag_metadata);
                }
            }
            _ => {
                // For unsupported metadata formats, use filename
                track.metadata.title = path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(|s| s.to_string());
            }
        }

        // Compute content hash for deduplication and move detection
        if let Err(e) = track.compute_content_hash() {
            // Log error but don't fail the entire track creation
            eprintln!("Warning: Failed to compute content hash for {}: {}", path.display(), e);
        }

        // Feature-gated duration probing using symphonia
        #[cfg(feature = "probe")]
        {
            if track.duration.is_none() {
                if let Ok(duration) = self.probe_duration_with_symphonia(path) {
                    track.duration = Some(duration);
                    track.metadata.duration_ms = Some(duration.as_millis() as u64);
                }
            }
        }

        Ok(track)
    }

    fn extract_id3_metadata(&self, path: &Path) -> Result<TrackMetadata> {
        let tag = id3::Tag::read_from_path(path)?;
        Ok(TrackMetadata::from_id3_tag(&tag))
    }

    fn extract_mp4_metadata(&self, path: &Path) -> Result<TrackMetadata> {
        let tag = mp4ameta::Tag::read_from_path(path)?;
        
        Ok(TrackMetadata {
            title: tag.title().map(|s| s.to_string()),
            artist: tag.artist().map(|s| s.to_string()),
            album: tag.album().map(|s| s.to_string()),
            album_artist: tag.album_artist().map(|s| s.to_string()),
            track_number: tag.track_number().map(|t| t as u32),
            disc_number: tag.disc_number().map(|d| d as u32),
            year: tag.year().and_then(|y| y.parse().ok()),
            genre: tag.genre().map(|s| s.to_string()),
            duration_ms: tag.duration().map(|d| d.as_millis() as u64),
        })
    }

    fn extract_flac_metadata(&self, _path: &Path) -> Result<TrackMetadata> {
        // For now, return empty metadata for FLAC
        // TODO: Implement FLAC metadata extraction
        Ok(TrackMetadata::default())
    }

    /// Feature-gated duration probing using symphonia codec
    #[cfg(feature = "probe")]
    fn probe_duration_with_symphonia(&self, path: &Path) -> Result<std::time::Duration> {
        use symphonia::core::formats::FormatOptions;
        use symphonia::core::io::MediaSourceStream;
        use symphonia::core::meta::MetadataOptions;
        use symphonia::core::probe::Hint;
        use std::fs::File;
        use std::time::Duration;

        let file = File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            hint.with_extension(extension);
        }

        let meta_opts: MetadataOptions = Default::default();
        let fmt_opts: FormatOptions = Default::default();

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &fmt_opts, &meta_opts)?;

        let mut format = probed.format;
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
            .ok_or_else(|| anyhow::anyhow!("No supported audio tracks found"))?;

        // Calculate duration from time base and frame count
        if let (Some(time_base), Some(n_frames)) = (track.codec_params.time_base, track.codec_params.n_frames) {
            let duration_secs = time_base.calc_time(n_frames).seconds as f64 
                + (time_base.calc_time(n_frames).frac as f64 / time_base.denom as f64);
            return Ok(Duration::from_secs_f64(duration_secs));
        }

        // Fallback: try to get duration from format metadata
        if let Some(metadata) = format.metadata().current() {
            for tag in metadata.tags() {
                if tag.key == "DURATION" || tag.key == "LENGTH" {
                    if let Ok(duration_ms) = tag.value.to_string().parse::<u64>() {
                        return Ok(Duration::from_millis(duration_ms));
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Could not determine duration from file"))
    }
}

impl Default for MusicScanner {
    fn default() -> Self {
        Self::new()
    }
}
