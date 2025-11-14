use super::TrackBehavior;
use chrono::{DateTime, Utc};
use rand::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

pub struct WeightCalculator {
    decay_days: u64,
    boost_factor: f64,
    penalty_factor: f64,
}

impl WeightCalculator {
    pub fn new(decay_days: u64) -> Self {
        Self {
            decay_days,
            boost_factor: 1.5,
            penalty_factor: 0.3,
        }
    }
    
    pub fn calculate_weight(&self, behavior: &TrackBehavior, current_time: DateTime<Utc>) -> f64 {
        let mut weight = 1.0;
        
        // Time-based decay/boost
        if let Some(last_played) = behavior.last_played {
            let days_since = (current_time - last_played).num_days() as u64;
            
            if days_since > self.decay_days {
                // Boost tracks that haven't been played recently
                let boost = (days_since as f64 / self.decay_days as f64).min(3.0);
                weight *= 1.0 + (boost * 0.2);
            } else if days_since < 1 {
                // Slightly reduce weight for recently played tracks
                weight *= 0.8;
            }
        } else {
            // Boost unplayed tracks
            weight *= 1.3;
        }
        
        // Completion rate influence
        if behavior.completion_rate > 80.0 {
            weight *= self.boost_factor;
        } else if behavior.completion_rate < 30.0 {
            weight *= self.penalty_factor;
        }
        
        // Skip ratio influence
        if behavior.total_plays > 0 {
            let skip_ratio = behavior.total_skips as f64 / behavior.total_plays as f64;
            weight *= (1.0 - skip_ratio * 0.6).max(0.2);
        }
        
        // Tag-based adjustments
        for tag in &behavior.tags {
            match tag.as_str() {
                "favorite" => weight *= 1.8,
                "often_skipped" => weight *= 0.2,
                "skip_early" => weight *= 0.4,
                "frequently_played" => {
                    // Slight penalty to encourage variety
                    weight *= 0.9;
                }
                "high_skip_rate" => weight *= 0.3,
                "low_skip_rate" => weight *= 1.2,
                _ => {}
            }
        }
        
        // Ensure weight stays within reasonable bounds
        weight.max(0.05).min(5.0)
    }
}

pub struct ShuffleWeighting {
    calculator: WeightCalculator,
    rng: ThreadRng,
}

impl ShuffleWeighting {
    pub fn new(decay_days: u64) -> Self {
        Self {
            calculator: WeightCalculator::new(decay_days),
            rng: thread_rng(),
        }
    }
    
    /// Select next track using weighted random selection
    pub fn select_next_track(
        &mut self,
        available_tracks: &[Uuid],
        behaviors: &HashMap<Uuid, TrackBehavior>,
        recently_played: &[Uuid], // tracks to avoid
    ) -> Option<Uuid> {
        if available_tracks.is_empty() {
            return None;
        }
        
        let current_time = Utc::now();
        let mut weighted_tracks = Vec::new();
        
        for &track_id in available_tracks {
            // Skip recently played tracks unless it's the only option
            if recently_played.contains(&track_id) && available_tracks.len() > recently_played.len() {
                continue;
            }
            
            let weight = if let Some(behavior) = behaviors.get(&track_id) {
                self.calculator.calculate_weight(behavior, current_time)
            } else {
                // New tracks get neutral weight with slight boost
                1.2
            };
            
            weighted_tracks.push((track_id, weight));
        }
        
        if weighted_tracks.is_empty() {
            // Fallback to any available track
            return available_tracks.choose(&mut self.rng).copied();
        }
        
        // Weighted random selection
        self.weighted_random_select(&weighted_tracks)
    }
    
    /// Generate a shuffled playlist using intelligent weighting
    pub fn generate_shuffled_playlist(
        &mut self,
        all_tracks: &[Uuid],
        behaviors: &HashMap<Uuid, TrackBehavior>,
        playlist_size: usize,
    ) -> Vec<Uuid> {
        let mut playlist = Vec::new();
        let mut available = all_tracks.to_vec();
        let mut recently_played = Vec::new();
        
        for _ in 0..playlist_size.min(all_tracks.len()) {
            if let Some(selected) = self.select_next_track(&available, behaviors, &recently_played) {
                playlist.push(selected);
                
                // Remove from available and add to recently played
                available.retain(|&id| id != selected);
                recently_played.push(selected);
                
                // Keep recently played list manageable
                if recently_played.len() > (all_tracks.len() / 4).max(5) {
                    recently_played.remove(0);
                }
                
                // If we've used all tracks, reset available but keep recently played
                if available.is_empty() && playlist.len() < playlist_size {
                    available = all_tracks.to_vec();
                    available.retain(|id| !recently_played.contains(id));
                }
            } else {
                break;
            }
        }
        
        playlist
    }
    
    fn weighted_random_select(&mut self, weighted_tracks: &[(Uuid, f64)]) -> Option<Uuid> {
        let total_weight: f64 = weighted_tracks.iter().map(|(_, weight)| weight).sum();
        
        if total_weight <= 0.0 {
            return weighted_tracks.choose(&mut self.rng).map(|(id, _)| *id);
        }
        
        let mut random_value = self.rng.gen::<f64>() * total_weight;
        
        for &(track_id, weight) in weighted_tracks {
            random_value -= weight;
            if random_value <= 0.0 {
                return Some(track_id);
            }
        }
        
        // Fallback (shouldn't happen with proper weights)
        weighted_tracks.last().map(|(id, _)| *id)
    }
    
    /// Update weights for all tracks based on current behavior
    pub fn recalculate_all_weights(
        &self,
        behaviors: &mut HashMap<Uuid, TrackBehavior>,
    ) {
        let current_time = Utc::now();
        
        for behavior in behaviors.values_mut() {
            behavior.weight = self.calculator.calculate_weight(behavior, current_time);
        }
    }
    
    /// Get tracks sorted by current weight (for debugging/analysis)
    pub fn get_tracks_by_weight(
        &self,
        behaviors: &HashMap<Uuid, TrackBehavior>,
    ) -> Vec<(Uuid, f64)> {
        let current_time = Utc::now();
        let mut weighted_tracks: Vec<_> = behaviors
            .iter()
            .map(|(&id, behavior)| {
                let weight = self.calculator.calculate_weight(behavior, current_time);
                (id, weight)
            })
            .collect();
        
        weighted_tracks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        weighted_tracks
    }
}
