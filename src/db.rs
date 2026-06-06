use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use directories::ProjectDirs;
use rusqlite::{Connection, OptionalExtension};
use std::path::PathBuf;

/// Returns the default database path following XDG spec:
/// $XDG_DATA_HOME/nutlog/nutlog.db or ~/.local/share/nutlog/nutlog.db
pub fn default_db_path() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("com", "nutlog", "nutlog") {
        let mut path = proj_dirs.data_dir().to_path_buf();
        path.push("nutlog.db");
        path
    } else {
        // Fallback for unusual envs
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp"));
        let mut path = home;
        path.push(".local/share/nutlog/nutlog.db");
        path
    }
}

/// Resolve DB path: use override if provided, else default.
/// Ensures parent directory exists.
pub fn resolve_db_path(override_path: Option<&str>) -> Result<PathBuf> {
    let path = match override_path {
        Some(p) => PathBuf::from(p),
        None => default_db_path(),
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!("failed to create database directory: {}", parent.display())
        })?;
    }
    Ok(path)
}

/// Open (or create) the SQLite DB, run migrations, return connection.
pub fn open_db(override_path: Option<&str>) -> Result<Connection> {
    let path = resolve_db_path(override_path)?;
    let conn = Connection::open(&path)
        .with_context(|| format!("failed to open database at {}", path.display()))?;
    // Enable foreign keys
    conn.execute("PRAGMA foreign_keys = ON", [])?;
    run_migrations(&conn)?;
    Ok(conn)
}

fn run_migrations(conn: &Connection) -> Result<()> {
    let current_version: i32 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .optional()?
        .unwrap_or(0);

    let migrations = [
        // v1: core tables
        r#"
        CREATE TABLE IF NOT EXISTS products (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS nutrients (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            unit TEXT NOT NULL,
            recommended_intake REAL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS product_tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS product_tag_associations (
            product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
            tag_id INTEGER NOT NULL REFERENCES product_tags(id) ON DELETE CASCADE,
            PRIMARY KEY (product_id, tag_id)
        );

        CREATE TABLE IF NOT EXISTS stores (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS store_tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS store_tag_associations (
            store_id INTEGER NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
            tag_id INTEGER NOT NULL REFERENCES store_tags(id) ON DELETE CASCADE,
            PRIMARY KEY (store_id, tag_id)
        );

        CREATE TABLE IF NOT EXISTS purchases (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
            quantity REAL NOT NULL DEFAULT 1.0,
            price_cents INTEGER,
            store_id INTEGER REFERENCES stores(id) ON DELETE SET NULL,
            purchased_at TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS consumptions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
            quantity REAL NOT NULL,
            unit TEXT,
            consumed_at TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS product_nutritions (
            product_id INTEGER PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,
            reference_quantity REAL NOT NULL,
            reference_unit TEXT NOT NULL,
            energy_kcal REAL,
            protein_g REAL,
            carbohydrates_g REAL,
            fat_g REAL,
            fiber_g REAL,
            sugars_g REAL
        );

        CREATE TABLE IF NOT EXISTS product_micronutrients (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
            nutrient_id INTEGER NOT NULL REFERENCES nutrients(id) ON DELETE CASCADE,
            amount REAL NOT NULL,
            unit TEXT NOT NULL,
            UNIQUE(product_id, nutrient_id)
        );

        CREATE INDEX IF NOT EXISTS idx_purchases_product ON purchases(product_id);
        CREATE INDEX IF NOT EXISTS idx_purchases_purchased_at ON purchases(purchased_at);
        CREATE INDEX IF NOT EXISTS idx_consumptions_product ON consumptions(product_id);
        CREATE INDEX IF NOT EXISTS idx_consumptions_consumed_at ON consumptions(consumed_at);
        "#,
        // v2: pre-populate common nutrients (idempotent)
        r#"
        INSERT OR IGNORE INTO nutrients (name, unit, recommended_intake, created_at)
        VALUES
            ('Protein', 'g', 50.0, '2026-01-01T00:00:00Z'),
            ('Carbohydrates', 'g', 300.0, '2026-01-01T00:00:00Z'),
            ('Fat', 'g', 70.0, '2026-01-01T00:00:00Z'),
            ('Fiber', 'g', 25.0, '2026-01-01T00:00:00Z'),
            ('Sugars', 'g', NULL, '2026-01-01T00:00:00Z'),
            ('Vitamin C', 'mg', 90.0, '2026-01-01T00:00:00Z'),
            ('Vitamin D', 'µg', 15.0, '2026-01-01T00:00:00Z'),
            ('Calcium', 'mg', 1000.0, '2026-01-01T00:00:00Z'),
            ('Iron', 'mg', 18.0, '2026-01-01T00:00:00Z'),
            ('Potassium', 'mg', 4700.0, '2026-01-01T00:00:00Z');
        "#,
        // v3: common supplement / active-compound nutrients (idempotent).
        // These make the examples in spec/04-expand-micronutrients.md work immediately
        // and address the "pre-populated nutrients insufficient for supplements" gap.
        r#"
        INSERT OR IGNORE INTO nutrients (name, unit, recommended_intake, created_at)
        VALUES
            ('Creatine Monohydrate', 'g', NULL, '2026-01-01T00:00:00Z'),
            ('Omega 3 EPA', 'mg', NULL, '2026-01-01T00:00:00Z'),
            ('Omega 3 DHA', 'mg', NULL, '2026-01-01T00:00:00Z'),
            ('Magnesium elemental', 'mg', 420.0, '2026-01-01T00:00:00Z'),
            ('Collagen peptides', 'g', NULL, '2026-01-01T00:00:00Z'),
            ('Hyaluronic acid', 'mg', NULL, '2026-01-01T00:00:00Z');
        "#,
    ];

    for (i, sql) in migrations.iter().enumerate() {
        let target_version = (i + 1) as i32;
        if current_version < target_version {
            conn.execute_batch(sql)
                .with_context(|| format!("migration v{} failed", target_version))?;
            conn.execute(&format!("PRAGMA user_version = {}", target_version), [])?;
        }
    }

    Ok(())
}

/// Helper to get current UTC timestamp as ISO string.
pub fn now_utc() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Parse a flexible date string into UTC datetime at midnight-ish for date purposes.
/// Supports: today, yesterday, YYYY-MM-DD, and simple relatives.
/// Returns the UTC instant corresponding to start of day in local? For storage we store the date as provided in UTC context?
/// Per spec: stored in UTC. For input "today" means current local? But to simplify, treat natural as local day start converted to UTC?
/// For simplicity in v1: parse to NaiveDate, then assume UTC midnight for that date.
/// Better: use local for "today" etc, convert the intended wall time to UTC for storage? But dates for logs are "the day".
/// Decision: for purchase/consumption dates, we store the instant, but for --date we interpret "today" etc relative to local time, store as UTC of that local midnight?
/// To keep practical: parse to a UTC DateTime representing the start of the logical day.
pub fn parse_flexible_date(s: &str) -> Result<DateTime<Utc>> {
    let s = s.trim().to_lowercase();
    let now = chrono::Local::now();
    let today = now.date_naive();

    let naive = if s == "today" {
        today
    } else if s == "yesterday" {
        today - chrono::Duration::days(1)
    } else if s == "tomorrow" {
        today + chrono::Duration::days(1)
    } else if let Ok(d) = chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d") {
        d
    } else if s.ends_with(" days ago") || s.ends_with(" day ago") {
        // crude "3 days ago"
        if let Some(num_str) = s.split_whitespace().next() {
            if let Ok(n) = num_str.parse::<i64>() {
                today - chrono::Duration::days(n)
            } else {
                return Err(anyhow!("unrecognized date: {}", s));
            }
        } else {
            return Err(anyhow!("unrecognized date: {}", s));
        }
    } else if s == "last week" {
        today - chrono::Duration::days(7)
    } else if s == "last month" {
        today - chrono::Duration::days(30)
    } else {
        // try other formats
        if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
            return Ok(dt.with_timezone(&Utc));
        }
        if let Ok(d) = chrono::NaiveDate::parse_from_str(&s, "%m-%d-%Y") {
            d
        } else if let Ok(d) = chrono::NaiveDate::parse_from_str(&s, "%d-%m-%Y") {
            d
        } else {
            return Err(anyhow!(
                "unrecognized date format: '{}'. Use today, yesterday, 2026-05-20, etc.",
                s
            ));
        }
    };

    // Treat the date as local midnight, convert to UTC.
    let local_dt = naive
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| anyhow!("invalid date"))?
        .and_local_timezone(chrono::Local)
        .single()
        .ok_or_else(|| anyhow!("ambiguous local time for date"))?;
    Ok(local_dt.with_timezone(&Utc))
}

/// Format a stored UTC timestamp for human output in local zone.
pub fn format_local(ts: &str) -> String {
    if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
        let local = dt.with_timezone(&chrono::Local);
        local.format("%Y-%m-%d %H:%M:%S %Z").to_string()
    } else {
        ts.to_string()
    }
}

/// For JSON, return both.
#[derive(serde::Serialize, Debug, Clone)]
pub struct TimestampInfo {
    pub utc: String,
    pub local: String,
}

pub fn make_timestamp_info(ts: &str) -> TimestampInfo {
    let utc = ts.to_string();
    let local = if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
        dt.with_timezone(&chrono::Local)
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    } else {
        ts.to_string()
    };
    TimestampInfo { utc, local }
}
