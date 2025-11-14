use super::{BehaviorDatabase, PlaySession, TrackBehavior};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkipReason {
    UserSkip,
    NextTrack,
    PreviousTrack,
    PlaylistEnd,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlaybackEvent {
    TrackStarted {
        track_id: Uuid,
        timestamp: DateTime<Utc>,
    },
    TrackPaused {
        track_id: Uuid,
        position: u64, // seconds
        timestamp: DateTime<Utc>,
    },
    TrackResumed {
        track_id: Uuid,
        position: u64,
        timestamp: DateTime<Utc>,
    },
    TrackSkipped {
        track_id: Uuid,
        position: u64,
        reason: SkipReason,
        timestamp: DateTime<Utc>,
    },
    TrackCompleted {
        track_id: Uuid,
        timestamp: DateTime<Utc>,
    },
}

pub struct BehaviorTracker {
    database: BehaviorDatabase,
    current_session: Option<ActiveSession>,
    min_play_time: u64, // minimum seconds to count as a "play"
}

#[derive(Debug)]
struct ActiveSession {
    session: PlaySession,
    actual_play_time: u64, // time actually spent playing (excluding pauses)
    pause_start: Option<DateTime<Utc>>,
}

impl BehaviorTracker {
    pub fn new(database: BehaviorDatabase, min_play_time: u64) -> Self {
        Self {
            database,
            current_session: None,
            min_play_time,
        }
    }
    
    pub async fn handle_event(&mut self, event: PlaybackEvent) -> Result<()> {
        match event {
            PlaybackEvent::TrackStarted { track_id, timestamp } => {
                self.start_session(track_id, timestamp).await?;
            }
            PlaybackEvent::TrackPaused { track_id, position, timestamp } => {
                self.pause_session(track_id, position, timestamp)?;
            }
            PlaybackEvent::TrackResumed { track_id, position, timestamp } => {
                self.resume_session(track_id, position, timestamp)?;
            }
            PlaybackEvent::TrackSkipped { track_id, position, reason, timestamp } => {
                self.end_session(track_id, position, Some(reason), timestamp).await?;
            }
            PlaybackEvent::TrackCompleted { track_id, timestamp } => {
                // For completed tracks, we assume they played to the end
                if let Some(session) = &self.current_session {
                    let position = session.session.track_duration;
                    self.end_session(track_id, position, None, timestamp).await?;
                }
            }
        }
        
        Ok(())
    }
    
    async fn start_session(&mut self, track_id: Uuid, timestamp: DateTime<Utc>) -> Result<()> {
        // End any existing session first
        if let Some(active) = &self.current_session {
            let old_track_id = active.session.track_id;
            let position = active.actual_play_time;
            self.end_session(old_track_id, position, Some(SkipReason::NextTrack), timestamp).await?;
        }
        
        // Get track duration from database or estimate
        let track_duration = self.database.get_track_duration(track_id).await?
            .unwrap_or(180); // Default 3 minutes if unknown
        
        let session = PlaySession {
            session_id: Uuid::new_v4(),
            track_id,
            started_at: timestamp,
            ended_at: None,
            play_duration: 0,
            track_duration,
            skip_reason: None,
            completion_percentage: 0.0,
        };
        
        self.current_session = Some(ActiveSession {
            session,
            actual_play_time: 0,
            pause_start: None,
        });
        
        Ok(())
    }
    
    fn pause_session(&mut self, track_id: Uuid, position: u64, timestamp: DateTime<Utc>) -> Result<()> {
        if let Some(active) = &mut self.current_session {
            if active.session.track_id == track_id && active.pause_start.is_none() {
                active.actual_play_time = position;
                active.pause_start = Some(timestamp);
            }
        }
        Ok(())
    }
    
    fn resume_session(&mut self, track_id: Uuid, _position: u64, _timestamp: DateTime<Utc>) -> Result<()> {
        if let Some(active) = &mut self.current_session {
            if active.session.track_id == track_id {
                active.pause_start = None;
                // Note: We don't update position here as we track actual play time separately
            }
        }
        Ok(())
    }
    
    async fn end_session(
        &mut self,
        track_id: Uuid,
        position: u64,
        skip_reason: Option<SkipReason>,
        timestamp: DateTime<Utc>,
    ) -> Result<()> {
        if let Some(mut active) = self.current_session.take() {
            if active.session.track_id == track_id {
                // Update session with final data
                active.session.ended_at = Some(timestamp);
                active.session.play_duration = position.min(active.actual_play_time.max(position));
                active.session.skip_reason = skip_reason;
                active.session.completion_percentage = 
                    (active.session.play_duration as f64 / active.session.track_duration as f64 * 100.0).min(100.0);
                
                // Only record if played for minimum time
                if active.session.play_duration >= self.min_play_time {
                    self.record_session(active.session).await?;
                }
            }
        }
        
        Ok(())
    }
    
    async fn record_session(&mut self, session: PlaySession) -> Result<()> {
        // Save session to database
        self.database.save_session(&session).await?;
        
        // Update track behavior
        let mut behavior = self.database.get_track_behavior(session.track_id).await?
            .unwrap_or_else(|| TrackBehavior::new(session.track_id));
        
        behavior.update_from_session(&session);
        
        // Recalculate weight
        let days_since_last = behavior.last_played
            .map(|last| (Utc::now() - last).num_days() as u64);
        behavior.weight = behavior.calculate_shuffle_weight(days_since_last);
        
        self.database.save_track_behavior(&behavior).await?;
        
        Ok(())
    }
    
    pub async fn get_track_behavior(&self, track_id: Uuid) -> Result<Option<TrackBehavior>> {
        self.database.get_track_behavior(track_id).await
    }
    
    pub async fn get_all_behaviors(&self) -> Result<Vec<TrackBehavior>> {
        self.database.get_all_track_behaviors().await
    }
}
