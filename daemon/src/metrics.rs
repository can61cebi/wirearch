//! Usage metrics: hourly and daily upload/download rollups in SQLite.
//!
//! The sampler feeds reset-aware byte deltas; buckets are keyed by
//! `unix_seconds / 3600` (hour) and `/ 86400` (day).

use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

pub struct Metrics {
    conn: Mutex<Connection>,
}

impl Metrics {
    pub fn open(path: &Path) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             CREATE TABLE IF NOT EXISTS rollup_hour(
                 hour INTEGER PRIMARY KEY,
                 rx INTEGER NOT NULL DEFAULT 0,
                 tx INTEGER NOT NULL DEFAULT 0);
             CREATE TABLE IF NOT EXISTS rollup_day(
                 day INTEGER PRIMARY KEY,
                 rx INTEGER NOT NULL DEFAULT 0,
                 tx INTEGER NOT NULL DEFAULT 0);",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Add received/sent byte deltas to the current hour and day buckets.
    pub fn add(&self, rx: u64, tx: u64) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let hour = (now / 3600) as i64;
        let day = (now / 86400) as i64;
        let conn = self.conn.lock().unwrap();
        let _ = conn.execute(
            "INSERT INTO rollup_hour(hour, rx, tx) VALUES(?1, ?2, ?3)
             ON CONFLICT(hour) DO UPDATE SET rx = rx + ?2, tx = tx + ?3",
            rusqlite::params![hour, rx as i64, tx as i64],
        );
        let _ = conn.execute(
            "INSERT INTO rollup_day(day, rx, tx) VALUES(?1, ?2, ?3)
             ON CONFLICT(day) DO UPDATE SET rx = rx + ?2, tx = tx + ?3",
            rusqlite::params![day, rx as i64, tx as i64],
        );
    }

    /// Most recent `count` hourly buckets, oldest first.
    pub fn hourly(&self, count: u32) -> Vec<(i64, u64, u64)> {
        self.query("rollup_hour", "hour", count)
    }

    /// Most recent `count` daily buckets, oldest first.
    pub fn daily(&self, count: u32) -> Vec<(i64, u64, u64)> {
        self.query("rollup_day", "day", count)
    }

    fn query(&self, table: &str, col: &str, count: u32) -> Vec<(i64, u64, u64)> {
        // `table` and `col` are fixed literals from the callers above.
        let sql = format!("SELECT {col}, rx, tx FROM {table} ORDER BY {col} DESC LIMIT ?1");
        let conn = self.conn.lock().unwrap();
        let mut stmt = match conn.prepare(&sql) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let mapped = stmt.query_map([count as i64], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)? as u64,
                r.get::<_, i64>(2)? as u64,
            ))
        });
        let mut out: Vec<(i64, u64, u64)> = match mapped {
            Ok(rows) => rows.flatten().collect(),
            Err(_) => Vec::new(),
        };
        out.reverse();
        out
    }
}
