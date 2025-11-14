use regex::Regex;

#[derive(Debug, Clone)]
pub struct ParsedMetadata {
    pub suggested_title: String,
    pub suggested_artist: String,
    pub confidence: f32, // 0.0 to 1.0
    pub pattern_used: String,
    pub normalization_applied: Vec<String>, // Track what normalizations were applied
}

#[derive(Debug, Clone)]
pub struct DelimiterInfo {
    pub delimiter: String,
    pub confidence: f32,
    pub typical_pattern: String,
}

pub struct MetadataParser {
    patterns: Vec<ParsePattern>,
    delimiter_cache: std::collections::HashMap<String, DelimiterInfo>, // Cache common delimiters
}

#[derive(Debug, Clone)]
struct ParsePattern {
    name: String,
    regex: Regex,
    title_group: usize,
    artist_group: usize,
    confidence: f32,
}



impl MetadataParser {
    pub fn new() -> Self {
        let mut patterns = Vec::new();
        let mut delimiter_cache = std::collections::HashMap::new();
        
        // Pre-populate delimiter cache with common patterns for cheap normalization
        delimiter_cache.insert(" - ".to_string(), DelimiterInfo {
            delimiter: " - ".to_string(),
            confidence: 0.9,
            typical_pattern: "Artist - Title".to_string(),
        });
        delimiter_cache.insert(" – ".to_string(), DelimiterInfo {
            delimiter: " – ".to_string(), // Em dash
            confidence: 0.85,
            typical_pattern: "Artist – Title".to_string(),
        });
        delimiter_cache.insert(" | ".to_string(), DelimiterInfo {
            delimiter: " | ".to_string(),
            confidence: 0.8,
            typical_pattern: "Artist | Title".to_string(),
        });
        delimiter_cache.insert(" ~ ".to_string(), DelimiterInfo {
            delimiter: " ~ ".to_string(),
            confidence: 0.7,
            typical_pattern: "Artist ~ Title".to_string(),
        });
        
        // Ordered pipeline: cheap delimiter normalizations first, then expensive regexes
        // Pattern 1: "Number - Artist - Title (Extra Info).ext"
        // Example: "18 - Heavy Is the Crown (Official Audio) - Linkin Park.m4a"
        if let Ok(regex) = Regex::new(r"^\d+\s*-\s*(.+?)\s*\(.*?\)\s*-\s*(.+?)\.") {
            patterns.push(ParsePattern {
                name: "Number - Title (Info) - Artist".to_string(),
                regex,
                title_group: 1,
                artist_group: 2,
                confidence: 0.9,
            });
        }
        
        // Pattern 2: "Number - Artist - Title.ext"
        // Example: "21 - blink-182 - TAKE ME IN (Official Lyric Video).m4a"
        if let Ok(regex) = Regex::new(r"^\d+\s*-\s*(.+?)\s*-\s*(.+?)(?:\s*\(.*?\))?\.") {
            patterns.push(ParsePattern {
                name: "Number - Artist - Title".to_string(),
                regex,
                title_group: 2,
                artist_group: 1,
                confidence: 0.85,
            });
        }
        
        // Pattern 3: "Artist - Title (Extra).ext"
        // Example: "The Black Keys - Beautiful People (Stay High) (Official Video).m4a"
        if let Ok(regex) = Regex::new(r"^(.+?)\s*-\s*(.+?)(?:\s*\(.*?\))*\.") {
            patterns.push(ParsePattern {
                name: "Artist - Title".to_string(),
                regex,
                title_group: 2,
                artist_group: 1,
                confidence: 0.8,
            });
        }
        
        // Pattern 4: "Title - Artist.ext"
        if let Ok(regex) = Regex::new(r"^(.+?)\s*-\s*(.+?)\.") {
            patterns.push(ParsePattern {
                name: "Title - Artist (fallback)".to_string(),
                regex,
                title_group: 1,
                artist_group: 2,
                confidence: 0.6,
            });
        }
        
        // Pattern 5: Just filename without extension (lowest confidence)
        if let Ok(regex) = Regex::new(r"^(.+?)\.") {
            patterns.push(ParsePattern {
                name: "Filename only".to_string(),
                regex,
                title_group: 1,
                artist_group: 0, // Will use "Unknown Artist"
                confidence: 0.3,
            });
        }
        
        Self { patterns, delimiter_cache }
    }
    
    pub fn parse_filename(&self, filename: &str) -> ParsedMetadata {
        let mut normalizations_applied = Vec::new();
        
        // Phase 1: Cheap delimiter normalization (O(1) hash lookups)
        if let Some(delimiter_result) = self.try_cheap_delimiter_parsing(filename) {
            normalizations_applied.push("cheap_delimiter".to_string());
            return ParsedMetadata {
                suggested_title: delimiter_result.0,
                suggested_artist: delimiter_result.1,
                confidence: delimiter_result.2,
                pattern_used: delimiter_result.3,
                normalization_applied: normalizations_applied,
            };
        }
        
        // Phase 2: Expensive regex patterns (only if cheap parsing failed)
        normalizations_applied.push("regex_patterns".to_string());
        for pattern in &self.patterns {
            if let Some(captures) = pattern.regex.captures(filename) {
                let title = if pattern.title_group > 0 {
                    captures.get(pattern.title_group)
                        .map(|m| self.clean_text(m.as_str()))
                        .unwrap_or_else(|| "Unknown Title".to_string())
                } else {
                    "Unknown Title".to_string()
                };
                
                let artist = if pattern.artist_group > 0 {
                    captures.get(pattern.artist_group)
                        .map(|m| self.clean_text(m.as_str()))
                        .unwrap_or_else(|| "Unknown Artist".to_string())
                } else {
                    "Unknown Artist".to_string()
                };
                
                return ParsedMetadata {
                    suggested_title: title,
                    suggested_artist: artist,
                    confidence: pattern.confidence,
                    pattern_used: pattern.name.clone(),
                    normalization_applied: normalizations_applied,
                };
            }
        }
        
        // Fallback if no patterns match
        normalizations_applied.push("fallback".to_string());
        ParsedMetadata {
            suggested_title: filename.to_string(),
            suggested_artist: "Unknown Artist".to_string(),
            confidence: 0.1,
            pattern_used: "No pattern matched".to_string(),
            normalization_applied: normalizations_applied,
        }
    }
    
    /// Fast O(1) delimiter-based parsing - checks common delimiters first
    fn try_cheap_delimiter_parsing(&self, filename: &str) -> Option<(String, String, f32, String)> {
        // Remove file extension first
        let name_without_ext = if let Some(dot_pos) = filename.rfind('.') {
            &filename[..dot_pos]
        } else {
            filename
        };
        
        // Try each cached delimiter for fast parsing
        for (delimiter, info) in &self.delimiter_cache {
            if let Some(split_pos) = name_without_ext.find(delimiter) {
                let (left_part, right_part) = name_without_ext.split_at(split_pos);
                let right_part = &right_part[delimiter.len()..]; // Skip delimiter
                
                // Clean both parts
                let left_clean = self.clean_text(left_part);
                let right_clean = self.clean_text(right_part);
                
                // Skip if either part is too short or empty
                if left_clean.len() < 2 || right_clean.len() < 2 {
                    continue;
                }
                
                // Determine which is artist vs title based on common patterns
                let (title, artist, confidence) = self.determine_artist_title_order(&left_clean, &right_clean, info.confidence);
                
                return Some((title, artist, confidence, format!("Cheap delimiter: {} ({})", info.delimiter, info.typical_pattern)));
            }
        }
        
        None
    }
    
    /// Heuristic to determine which part is artist vs title
    fn determine_artist_title_order(&self, left: &str, right: &str, base_confidence: f32) -> (String, String, f32) {
        let mut confidence = base_confidence;
        
        // Heuristic 1: Numbers at start usually indicate track number, so format is likely "Number - Artist - Title"
        if left.chars().next().map_or(false, |c| c.is_numeric()) {
            // If left starts with number, it's probably track number, so right is likely artist
            confidence *= 0.9; // Slightly lower confidence due to ambiguity
            return (right.to_string(), left.to_string(), confidence);
        }
        
        // Heuristic 2: Longer text is often the title
        if right.len() > left.len() + (left.len() / 2) {
            confidence *= 1.1; // Higher confidence
            return (right.to_string(), left.to_string(), confidence);
        }
        
        // Heuristic 3: Common video/audio indicators suggest title
        let title_indicators = ["(Official", "(Audio)", "(Video)", "(Lyric", "[Official", "[Audio]"];
        if title_indicators.iter().any(|indicator| right.contains(indicator)) {
            confidence *= 1.2; // Much higher confidence
            return (right.to_string(), left.to_string(), confidence);
        }
        
        // Default: assume "Artist - Title" format (most common)
        (right.to_string(), left.to_string(), confidence)
    }

    fn clean_text(&self, text: &str) -> String {
        let mut cleaned = text.to_string();
        
        // Remove common prefixes/suffixes
        let removals = [
            "(Official Audio)",
            "(Official Video)",
            "(Official Music Video)",
            "(Official Lyric Video)",
            "(Audio)",
            "(Video)",
            "(Lyric Video)",
            "(Music Video)",
            "[Official Audio]",
            "[Official Video]",
            "[Audio]",
            "[Video]",
        ];
        
        for removal in &removals {
            cleaned = cleaned.replace(removal, "");
        }
        
        // Clean up extra whitespace and trim
        cleaned = cleaned.trim().to_string();
        while cleaned.contains("  ") {
            cleaned = cleaned.replace("  ", " ");
        }
        
        // Remove leading/trailing dashes and numbers
        cleaned = cleaned.trim_matches(|c: char| c == '-' || c == '_' || c.is_whitespace()).to_string();
        
        // Remove leading numbers (track numbers) - using char-safe operations
        if let Some(first_letter) = cleaned.chars().position(|c| c.is_alphabetic()) {
            if first_letter > 0 {
                let chars: Vec<char> = cleaned.chars().collect();
                let prefix: String = chars[..first_letter].iter().collect();
                if prefix.chars().all(|c| c.is_numeric() || c == '.' || c == ' ' || c == '-') {
                    let suffix: String = chars[first_letter..].iter().collect();
                    cleaned = suffix.trim().to_string();
                }
            }
        }
        
        cleaned
    }
    
    pub fn format_as_song_artist(&self, title: &str, artist: &str) -> String {
        format!("{} - {}", title.trim(), artist.trim())
    }
    
    /// Bulk processing with performance optimizations
    pub fn suggest_corrections(&self, filenames: &[String]) -> Vec<(String, ParsedMetadata)> {
        filenames
            .iter()
            .map(|filename| (filename.clone(), self.parse_filename(filename)))
            .collect()
    }
    
    /// Add custom delimiter patterns for extensibility
    pub fn add_custom_delimiter(&mut self, delimiter: String, confidence: f32, pattern_name: String) {
        self.delimiter_cache.insert(delimiter.clone(), DelimiterInfo {
            delimiter,
            confidence,
            typical_pattern: pattern_name,
        });
    }
    
    /// Get delimiter statistics for analysis
    pub fn get_delimiter_stats(&self, filenames: &[String]) -> std::collections::HashMap<String, usize> {
        let mut stats = std::collections::HashMap::new();
        
        for filename in filenames {
            for delimiter in self.delimiter_cache.keys() {
                if filename.contains(delimiter) {
                    *stats.entry(delimiter.clone()).or_insert(0) += 1;
                }
            }
        }
        
        stats
    }
    
    /// Batch normalize with progress tracking
    pub fn batch_normalize(&self, filenames: &[String]) -> Vec<ParsedMetadata> {
        filenames
            .iter()
            .map(|filename| self.parse_filename(filename))
            .collect()
    }
    
    /// Get normalization performance metrics
    pub fn get_performance_metrics(&self, filenames: &[String]) -> (usize, usize, f32) {
        let results = self.batch_normalize(filenames);
        let cheap_delimiter_count = results.iter()
            .filter(|r| r.normalization_applied.contains(&"cheap_delimiter".to_string()))
            .count();
        let regex_count = results.iter()
            .filter(|r| r.normalization_applied.contains(&"regex_patterns".to_string()))
            .count();
        let avg_confidence = results.iter()
            .map(|r| r.confidence)
            .sum::<f32>() / results.len() as f32;
            
        (cheap_delimiter_count, regex_count, avg_confidence)
    }
}

impl Default for MetadataParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parsing_patterns() {
        let parser = MetadataParser::new();
        
        // Test pattern 1: Number - Title (Info) - Artist
        let result = parser.parse_filename("18 - Heavy Is the Crown (Official Audio) - Linkin Park.m4a");
        assert_eq!(result.suggested_title, "Heavy Is the Crown");
        assert_eq!(result.suggested_artist, "Linkin Park");
        
        // Test pattern 2: Number - Artist - Title
        let result = parser.parse_filename("21 - blink-182 - TAKE ME IN (Official Lyric Video).m4a");
        assert_eq!(result.suggested_title, "TAKE ME IN");
        assert_eq!(result.suggested_artist, "blink-182");
        
        // Test pattern 3: Artist - Title
        let result = parser.parse_filename("The Black Keys - Beautiful People (Stay High) (Official Video).m4a");
        assert_eq!(result.suggested_title, "Beautiful People (Stay High)");
        assert_eq!(result.suggested_artist, "The Black Keys");
    }
    
    #[test]
    fn test_text_cleaning() {
        let parser = MetadataParser::new();
        
        let cleaned = parser.clean_text("Heavy Is the Crown (Official Audio)");
        assert_eq!(cleaned, "Heavy Is the Crown");
        
        let cleaned = parser.clean_text("  TAKE ME IN  (Official Lyric Video)  ");
        assert_eq!(cleaned, "TAKE ME IN");
    }
    
    #[test]
    fn test_format_song_artist() {
        let parser = MetadataParser::new();
        
        let formatted = parser.format_as_song_artist("Heavy Is the Crown", "Linkin Park");
        assert_eq!(formatted, "Heavy Is the Crown - Linkin Park");
    }
}
