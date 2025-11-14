// Behavior tracking - the "smart" part of BangTunes
// Learns what you like and skip, makes shuffle actually useful

pub mod database;  // SQLite storage for behavior data
pub mod tracker;   // tracks play sessions and skip patterns
pub mod weighting; // calculates shuffle weights based on behavior

pub use database::BehaviorDatabase;
pub use tracker::{BehaviorTracker, PlaybackEvent, SkipReason};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackBehavior {
    pub track_id: Uuid,
    pub total_plays: u64,
    pub total_skips: u64,
    pub total_play_time: u64, // in seconds
    pub last_played: Option<DateTime<Utc>>,
    pub skip_positions: Vec<u64>, // positions where skips occurred (in seconds)
    pub completion_rate: f64, // percentage of track typically played
    pub weight: f64, // current shuffle weight
    pub tags: Vec<String>, // behavior-based tags
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaySession {
    pub session_id: Uuid,
    pub track_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub play_duration: u64, // seconds actually played
    pub track_duration: u64, // total track length
    pub skip_reason: Option<SkipReason>,
    pub completion_percentage: f64,
}

impl TrackBehavior {
    pub fn new(track_id: Uuid) -> Self {
        Self {
            track_id,
            total_plays: 0,
            total_skips: 0,
            total_play_time: 0,
            last_played: None,
            skip_positions: Vec::new(),
            completion_rate: 0.0,
            weight: 1.0, // neutral starting weight
            tags: Vec::new(),
        }
    }
    
    pub fn update_from_session(&mut self, session: &PlaySession) {
        self.total_plays += 1;
        self.total_play_time += session.play_duration;
        self.last_played = Some(session.started_at);
        
        if session.skip_reason.is_some() {
            self.total_skips += 1;
            // Record skip position as percentage of track
            let skip_position = (session.play_duration as f64 / session.track_duration as f64 * 100.0) as u64;
            self.skip_positions.push(skip_position);
        }
        
        // Update completion rate (running average)
        let new_completion = session.completion_percentage;
        if self.total_plays == 1 {
            self.completion_rate = new_completion;
        } else {
            // Weighted average favoring recent plays
            self.completion_rate = (self.completion_rate * 0.7) + (new_completion * 0.3);
        }
        
        // Update behavior tags
        self.update_tags();
    }
    
    fn update_tags(&mut self) {
        self.tags.clear();
        
        // Tag based on completion rate
        if self.completion_rate > 90.0 {
            self.tags.push("favorite".to_string());
        } else if self.completion_rate < 30.0 {
            self.tags.push("often_skipped".to_string());
        }
        
        // Tag based on skip patterns
        if self.skip_positions.len() > 3 {
            let avg_skip_position: f64 = self.skip_positions.iter().map(|&x| x as f64).sum::<f64>() / self.skip_positions.len() as f64;
            
            if avg_skip_position < 25.0 {
                self.tags.push("skip_early".to_string());
            } else if avg_skip_position > 75.0 {
                self.tags.push("skip_late".to_string());
            }
        }
        
        // Tag based on play frequency
        if self.total_plays > 10 {
            self.tags.push("frequently_played".to_string());
        }
        
        // Tag based on skip ratio
        let skip_ratio = self.total_skips as f64 / self.total_plays as f64;
        if skip_ratio > 0.7 {
            self.tags.push("high_skip_rate".to_string());
        } else if skip_ratio < 0.2 {
            self.tags.push("low_skip_rate".to_string());
        }
    }
    
    pub fn calculate_shuffle_weight(&self, days_since_last_play: Option<u64>) -> f64 {
        let mut weight = 1.0;
        
        // Boost favorites
        if self.tags.contains(&"favorite".to_string()) {
            weight *= 1.5;
        }
        
        // Reduce weight for often skipped tracks
        if self.tags.contains(&"often_skipped".to_string()) {
            weight *= 0.3;
        }
        
        // Boost tracks that haven't been played recently
        if let Some(days) = days_since_last_play {
            if days > 7 {
                weight *= 1.0 + (days as f64 * 0.1).min(2.0); // Cap at 3x boost
            }
        }
        
        // Reduce weight for high skip rate tracks
        let skip_ratio = self.total_skips as f64 / self.total_plays.max(1) as f64;
        weight *= (1.0 - skip_ratio * 0.5).max(0.1); // Never go below 0.1
        
        weight.max(0.1).min(5.0) // Clamp between 0.1 and 5.0
    }
}
