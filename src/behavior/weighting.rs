// SPDX-License-Identifier: MIT
// Copyright (c) 2024 BangTunes Contributors

use super::TrackBehavior;
use chrono::{DateTime, Utc};
use rand::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

/// Computes a shuffle weight for a track from its listening behavior and time-decay rules.
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
        
        // Tag-based adjustments.
        // "favorite"        = explicit user star (toggle keybinding) → strongest boost.
        // "high_completion" = auto-detected from completion rate > 90% → smaller boost.
        for tag in &behavior.tags {
            match tag.as_str() {
                "favorite" => weight *= 1.8,
                "high_completion" => weight *= 1.3,
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
        weight.clamp(0.05, 5.0)
    }
}

/// Weighted random track selector — wraps `WeightCalculator` with an RNG to pick the next track.
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
    
    pub fn weighted_random_select(&mut self, weighted_tracks: &[(Uuid, f64)]) -> Option<Uuid> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behavior::TrackBehavior;
    use chrono::Duration as ChronoDuration;

    fn behavior(completion_rate: f64, total_plays: u64, total_skips: u64) -> TrackBehavior {
        TrackBehavior {
            track_id: Uuid::new_v4(),
            total_plays,
            total_skips,
            total_play_time: 0,
            last_played: None,
            skip_positions: vec![],
            completion_rate,
            weight: 1.0,
            tags: vec![],
        }
    }

    #[test]
    fn high_completion_beats_high_skip_rate() {
        let calc = WeightCalculator::new(7);
        let now = Utc::now();
        let loved = behavior(90.0, 10, 0);
        let skipped = behavior(10.0, 10, 9);
        assert!(
            calc.calculate_weight(&loved, now) > calc.calculate_weight(&skipped, now),
            "high completion rate should outweigh high skip rate"
        );
    }

    #[test]
    fn unplayed_track_outweighs_recently_played_equivalent() {
        let calc = WeightCalculator::new(7);
        let now = Utc::now();
        // Neutral completion rate so no completion penalty applies to either
        let unplayed = behavior(50.0, 0, 0); // never played → 1.3× time boost
        let mut recent = behavior(50.0, 5, 1);
        recent.last_played = Some(now - ChronoDuration::hours(1)); // played today → 0.8× penalty
        assert!(
            calc.calculate_weight(&unplayed, now) > calc.calculate_weight(&recent, now),
            "unplayed track (1.3× boost) should outscore recently-played equivalent (0.8× penalty)"
        );
    }

    #[test]
    fn favorite_tag_raises_weight_above_untagged() {
        let calc = WeightCalculator::new(7);
        let now = Utc::now();
        let mut fav = behavior(50.0, 5, 1);
        fav.tags = vec!["favorite".to_string()];
        let plain = behavior(50.0, 5, 1);
        assert!(
            calc.calculate_weight(&fav, now) > calc.calculate_weight(&plain, now),
            "favorite tag should apply 1.8× boost"
        );
    }

    #[test]
    fn often_skipped_tag_drops_weight_below_untagged() {
        let calc = WeightCalculator::new(7);
        let now = Utc::now();
        let mut penalised = behavior(50.0, 5, 1);
        penalised.tags = vec!["often_skipped".to_string()];
        let plain = behavior(50.0, 5, 1);
        assert!(
            calc.calculate_weight(&penalised, now) < calc.calculate_weight(&plain, now),
            "often_skipped tag should apply 0.2× penalty"
        );
    }

    #[test]
    fn stale_track_is_boosted_by_time_decay() {
        let calc = WeightCalculator::new(7);
        let now = Utc::now();
        let mut stale = behavior(50.0, 5, 1);
        stale.last_played = Some(now - ChronoDuration::days(30)); // 30 days ago, > decay_days
        let recent = behavior(50.0, 5, 1);
        assert!(
            calc.calculate_weight(&stale, now) > calc.calculate_weight(&recent, now),
            "track not played in >decay_days should be boosted"
        );
    }

    #[test]
    fn weight_is_clamped_between_0_05_and_5() {
        let calc = WeightCalculator::new(7);
        let now = Utc::now();
        // worst possible track: max skip rate + often_skipped tag
        let mut worst = behavior(0.0, 100, 100);
        worst.tags = vec!["often_skipped".to_string(), "high_skip_rate".to_string()];
        let w = calc.calculate_weight(&worst, now);
        assert!(w >= 0.05, "weight must not drop below 0.05 (got {w})");
        assert!(w <= 5.0, "weight must not exceed 5.0 (got {w})");
    }

    #[test]
    fn high_completion_tag_boosts_weight_above_untagged() {
        let calc = WeightCalculator::new(7);
        let now = Utc::now();
        let mut auto_tagged = behavior(50.0, 5, 1);
        auto_tagged.tags = vec!["high_completion".to_string()];
        let plain = behavior(50.0, 5, 1);
        assert!(
            calc.calculate_weight(&auto_tagged, now) > calc.calculate_weight(&plain, now),
            "high_completion tag should apply 1.3× boost"
        );
    }

    #[test]
    fn favorite_tag_not_auto_assigned_by_completion_rate() {
        // "favorite" must only come from explicit user action; WeightCalculator must
        // treat it as a separate signal from "high_completion".
        let calc = WeightCalculator::new(7);
        let now = Utc::now();
        let mut fav = behavior(50.0, 5, 1);
        fav.tags = vec!["favorite".to_string()];
        let mut auto = behavior(50.0, 5, 1);
        auto.tags = vec!["high_completion".to_string()];
        // User favorite (1.8×) must outweigh auto high_completion (1.3×)
        assert!(
            calc.calculate_weight(&fav, now) > calc.calculate_weight(&auto, now),
            "explicit favorite (1.8×) should score higher than auto high_completion (1.3×)"
        );
    }
}
