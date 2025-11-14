use super::{PlaySession, TrackBehavior};
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension, Row};
use std::path::Path;
use uuid::Uuid;

pub struct BehaviorDatabase {
    conn: Connection,
}

impl BehaviorDatabase {
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        let db = Self { conn };
        db.initialize_tables()?;
        Ok(db)
    }
    
    fn initialize_tables(&self) -> Result<()> {
        // Track behaviors table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS track_behaviors (
                track_id TEXT PRIMARY KEY,
                total_plays INTEGER NOT NULL DEFAULT 0,
                total_skips INTEGER NOT NULL DEFAULT 0,
                total_play_time INTEGER NOT NULL DEFAULT 0,
                last_played TEXT,
                skip_positions TEXT, -- JSON array
                completion_rate REAL NOT NULL DEFAULT 0.0,
                weight REAL NOT NULL DEFAULT 1.0,
                tags TEXT, -- JSON array
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        
        // Play sessions table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS play_sessions (
                session_id TEXT PRIMARY KEY,
                track_id TEXT NOT NULL,
                started_at TEXT NOT NULL,
                ended_at TEXT,
                play_duration INTEGER NOT NULL DEFAULT 0,
                track_duration INTEGER NOT NULL DEFAULT 0,
                skip_reason TEXT,
                completion_percentage REAL NOT NULL DEFAULT 0.0,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        
        // Track metadata table (for duration and other info)
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS track_metadata (
                track_id TEXT PRIMARY KEY,
                file_path TEXT,
                title TEXT,
                artist TEXT,
                album TEXT,
                duration INTEGER, -- seconds
                file_size INTEGER,
                last_modified TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        
        // Create indexes for performance
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sessions_track_id ON play_sessions(track_id)",
            [],
        )?;
        
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sessions_started_at ON play_sessions(started_at)",
            [],
        )?;
        
        Ok(())
    }
    
    pub async fn save_track_behavior(&self, behavior: &TrackBehavior) -> Result<()> {
        let skip_positions_json = serde_json::to_string(&behavior.skip_positions)?;
        let tags_json = serde_json::to_string(&behavior.tags)?;
        let last_played = behavior.last_played.map(|dt| dt.to_rfc3339());
        
        self.conn.execute(
            "INSERT OR REPLACE INTO track_behaviors 
             (track_id, total_plays, total_skips, total_play_time, last_played, 
              skip_positions, completion_rate, weight, tags, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, CURRENT_TIMESTAMP)",
            params![
                behavior.track_id.to_string(),
                behavior.total_plays,
                behavior.total_skips,
                behavior.total_play_time,
                last_played,
                skip_positions_json,
                behavior.completion_rate,
                behavior.weight,
                tags_json,
            ],
        )?;
        
        Ok(())
    }
    
    pub async fn get_track_behavior(&self, track_id: Uuid) -> Result<Option<TrackBehavior>> {
        let mut stmt = self.conn.prepare(
            "SELECT track_id, total_plays, total_skips, total_play_time, last_played,
                    skip_positions, completion_rate, weight, tags
             FROM track_behaviors WHERE track_id = ?1"
        )?;
        
        let behavior = stmt.query_row(params![track_id.to_string()], |row| {
            self.row_to_track_behavior(row)
        }).optional()?;
        
        Ok(behavior)
    }
    
    pub async fn get_all_track_behaviors(&self) -> Result<Vec<TrackBehavior>> {
        let mut stmt = self.conn.prepare(
            "SELECT track_id, total_plays, total_skips, total_play_time, last_played,
                    skip_positions, completion_rate, weight, tags
             FROM track_behaviors ORDER BY weight DESC"
        )?;
        
        let behaviors = stmt.query_map([], |row| {
            self.row_to_track_behavior(row)
        })?
        .collect::<Result<Vec<_>, _>>()?;
        
        Ok(behaviors)
    }
    
    pub async fn save_session(&self, session: &PlaySession) -> Result<()> {
        let skip_reason_str = session.skip_reason.as_ref()
            .map(|r| serde_json::to_string(r).unwrap_or_default());
        let ended_at = session.ended_at.map(|dt| dt.to_rfc3339());
        
        self.conn.execute(
            "INSERT INTO play_sessions 
             (session_id, track_id, started_at, ended_at, play_duration, 
              track_duration, skip_reason, completion_percentage)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                session.session_id.to_string(),
                session.track_id.to_string(),
                session.started_at.to_rfc3339(),
                ended_at,
                session.play_duration,
                session.track_duration,
                skip_reason_str,
                session.completion_percentage,
            ],
        )?;
        
        Ok(())
    }
    
    pub async fn get_track_duration(&self, track_id: Uuid) -> Result<Option<u64>> {
        let mut stmt = self.conn.prepare(
            "SELECT duration FROM track_metadata WHERE track_id = ?1"
        )?;
        
        let duration = stmt.query_row(params![track_id.to_string()], |row| {
            Ok(row.get::<_, Option<i64>>(0)?.map(|d| d as u64))
        }).optional()?.flatten();
        
        Ok(duration)
    }
    
    pub async fn save_track_metadata(
        &self,
        track_id: Uuid,
        file_path: &str,
        title: Option<&str>,
        artist: Option<&str>,
        album: Option<&str>,
        duration: Option<u64>,
        file_size: Option<u64>,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO track_metadata 
             (track_id, file_path, title, artist, album, duration, file_size, last_modified)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, CURRENT_TIMESTAMP)",
            params![
                track_id.to_string(),
                file_path,
                title,
                artist,
                album,
                duration.map(|d| d as i64),
                file_size.map(|s| s as i64),
            ],
        )?;
        
        Ok(())
    }
    
    fn row_to_track_behavior(&self, row: &Row) -> rusqlite::Result<TrackBehavior> {
        let track_id_str: String = row.get(0)?;
        let track_id = Uuid::parse_str(&track_id_str)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))?;
        
        let skip_positions_json: String = row.get(5)?;
        let skip_positions: Vec<u64> = serde_json::from_str(&skip_positions_json).unwrap_or_default();
        
        let tags_json: String = row.get(8)?;
        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
        
        let last_played_str: Option<String> = row.get(4)?;
        let last_played = last_played_str
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));
        
        Ok(TrackBehavior {
            track_id,
            total_plays: row.get(1)?,
            total_skips: row.get(2)?,
            total_play_time: row.get(3)?,
            last_played,
            skip_positions,
            completion_rate: row.get(6)?,
            weight: row.get(7)?,
            tags,
        })
    }
}
