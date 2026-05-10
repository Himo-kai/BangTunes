// SPDX-License-Identifier: MIT
// Copyright (c) 2024 BangTunes Contributors

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
        // "favorite" is set exclusively by explicit user action and must survive session updates.
        // All other tags are auto-derived from listening data and safe to recompute.
        let is_favorite = self.tags.contains(&"favorite".to_string());

        self.tags.clear();

        if is_favorite {
            self.tags.push("favorite".to_string());
        }

        // Tag based on completion rate.
        // NOTE: "high_completion" is auto-detected from listening data.
        //       "favorite" is reserved exclusively for explicit user action (toggle keybinding).
        if self.completion_rate > 90.0 {
            self.tags.push("high_completion".to_string());
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

        // Boost user-starred tracks and auto-detected high-completion tracks separately.
        // "favorite" = explicit user toggle; "high_completion" = auto-tag from listening data.
        if self.tags.contains(&"favorite".to_string()) {
            weight *= 1.5;
        }
        if self.tags.contains(&"high_completion".to_string()) {
            weight *= 1.2;
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
        
        weight.clamp(0.1, 5.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_session(completion: f64, skipped: bool) -> PlaySession {
        PlaySession {
            session_id: Uuid::new_v4(),
            track_id: Uuid::new_v4(),
            started_at: chrono::Utc::now(),
            ended_at: None,
            play_duration: (completion * 180.0 / 100.0) as u64,
            track_duration: 180,
            skip_reason: if skipped { Some(SkipReason::UserSkip) } else { None },
            completion_percentage: completion,
        }
    }

    #[test]
    fn auto_tag_uses_high_completion_not_favorite() {
        let mut behavior = TrackBehavior::new(Uuid::new_v4());
        // Drive completion rate above the 90% threshold
        behavior.update_from_session(&make_session(95.0, false));
        assert!(
            behavior.tags.contains(&"high_completion".to_string()),
            "expected 'high_completion' tag for >90% completion, got: {:?}",
            behavior.tags
        );
        assert!(
            !behavior.tags.contains(&"favorite".to_string()),
            "'favorite' must not be auto-assigned — it is reserved for explicit user action"
        );
    }

    #[test]
    fn favorite_tag_survives_session_update() {
        let mut behavior = TrackBehavior::new(Uuid::new_v4());
        // User explicitly marks favorite
        behavior.tags.push("favorite".to_string());
        // Track gets played (triggers update_tags internally)
        behavior.update_from_session(&make_session(50.0, false));
        assert!(
            behavior.tags.contains(&"favorite".to_string()),
            "favorite tag must survive update_from_session, got: {:?}",
            behavior.tags
        );
    }

    #[test]
    fn often_skipped_tag_applied_below_30_percent() {
        let mut behavior = TrackBehavior::new(Uuid::new_v4());
        behavior.update_from_session(&make_session(20.0, true));
        assert!(
            behavior.tags.contains(&"often_skipped".to_string()),
            "expected 'often_skipped' tag for <30% completion, got: {:?}",
            behavior.tags
        );
    }
}
