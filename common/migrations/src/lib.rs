use std::{collections::HashMap, hash::Hasher};

use rusqlite::{
    functions::{Aggregate, FunctionFlags},
    params, Connection,
};
use seahash::SeaHasher;
use sha2::{Digest, Sha256};

pub use rusqlite;

const CHANGELOG_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS database_changelog (
  version TEXT PRIMARY KEY,
  sha256 TEXT NOT NULL,
  created_at DEFAULT CURRENT_TIMESTAMP
);
"#;

#[derive(thiserror::Error, Debug)]
pub enum MigrationError {
    #[error("HashMismatch for version={version}: {expected} != {actual} (expected != actual)")]
    HashMismatch {
        expected: String,
        actual: String,
        version: String,
    },
    #[error("Database {0}")]
    Database(#[from] rusqlite::Error),
}

pub fn apply(conn: &Connection, changelog: &[(&str, &str)]) -> Result<(), MigrationError> {
    conn.execute(CHANGELOG_TABLE, params![])?;
    let mut stmt = conn
        .prepare("SELECT * FROM database_changelog ORDER BY created_at")
        .unwrap();
    let mut rows = stmt.query(params![])?;

    let mut applied_migrations = HashMap::new();
    while let Some(row) = rows.next()? {
        let version: String = row.get(0)?;
        let sha256: String = row.get(1)?;
        applied_migrations.insert(version, sha256);
    }

    for (version, sql) in changelog {
        let hash_bytes = Sha256::digest(sql);
        let hash = base16ct::lower::encode_string(&hash_bytes);

        if let Some(existing_hash) = applied_migrations.get(*version) {
            if existing_hash != &hash {
                return Err(MigrationError::HashMismatch {
                    expected: hash,
                    actual: existing_hash.clone(),
                    version: (*version).to_owned(),
                });
            }
            continue;
        }

        tracing::info!("Applying migration {}", version);
        conn.execute_batch(sql)?;
        conn.execute(
            "INSERT INTO database_changelog (version, sha256) VALUES (? ,?)",
            params![version, hash],
        )?;
    }

    let last_version = changelog.last().map(|(v, _)| *v);
    tracing::info!(last_version, "Database migrated successfully.");
    Ok(())
}

/// Add seahash scalar function to SQLite
pub fn add_seahash(conn: &Connection) -> Result<(), MigrationError> {
    conn.create_scalar_function(
        "seahash",
        1,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        seahash_scalar,
    )?;
    Ok(())
}

/// Add group_seahash aggregate function to SQLite
pub fn add_group_seahash(conn: &Connection) -> Result<(), MigrationError> {
    conn.create_aggregate_function(
        "group_seahash",
        1,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        SeaHashAggregate::default(),
    )?;
    Ok(())
}

/// SeaHash scalar function. It outputs a string because SQLite doesn't fully support u64 values.
fn seahash_scalar(ctx: &rusqlite::functions::Context<'_>) -> rusqlite::Result<String> {
    if ctx.len() != 1 {
        return Err(rusqlite::Error::UserFunctionError(Box::new(
            SeaHashError::ExpectedOneArgument,
        )));
    }

    let mut hasher = SeaHasher::new();
    let s = ctx
        .get_raw(0)
        .as_str()
        .map_err(|e| rusqlite::Error::UserFunctionError(e.into()))?;
    hasher.write(s.as_bytes());
    Ok(hasher.finish().to_string())
}

/// SeaHash aggregate function. It outputs a string because SQLite doesn't fully support u64 values.
#[derive(Default)]
struct SeaHashAggregate {}

impl Aggregate<SeaHasher, Option<String>> for SeaHashAggregate {
    fn init(&self, _ctx: &mut rusqlite::functions::Context<'_>) -> rusqlite::Result<SeaHasher> {
        Ok(SeaHasher::new())
    }

    fn step(
        &self,
        ctx: &mut rusqlite::functions::Context<'_>,
        hasher: &mut SeaHasher,
    ) -> rusqlite::Result<()> {
        if ctx.len() != 1 {
            return Err(rusqlite::Error::UserFunctionError(Box::new(
                SeaHashError::ExpectedOneArgument,
            )));
        }

        let s = ctx
            .get_raw(0)
            .as_str()
            .map_err(|e| rusqlite::Error::UserFunctionError(e.into()))?;
        hasher.write(s.as_bytes());
        Ok(())
    }

    fn finalize(
        &self,
        _: &mut rusqlite::functions::Context<'_>,
        hasher: Option<SeaHasher>,
    ) -> rusqlite::Result<Option<String>> {
        Ok(hasher.map(|h| h.finish().to_string()))
    }
}

#[derive(thiserror::Error, Debug)]
enum SeaHashError {
    #[error("Expected one argument to function seahash")]
    ExpectedOneArgument,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connect() -> Connection {
        Connection::open_in_memory().expect("Connection in memory")
    }

    fn assert_db_changelog(conn: &mut Connection, expected: Vec<(String, String)>) {
        let mut stmt = conn
            .prepare("SELECT * FROM database_changelog ORDER BY created_at")
            .expect("Prepare select changelog");
        let mut rows = stmt.query(params![]).expect("Select changelog");
        let mut entries = vec![];
        while let Some(row) = rows.next().expect("Next row") {
            let version: String = row.get(0).expect("Row version");
            let hash: String = row.get(1).expect("Row hash");
            entries.push((version, hash));
        }

        assert_eq!(entries, expected,);
    }

    #[test]
    fn migrations_empty() {
        let mut conn = connect();
        apply(&mut conn, &[]).expect("Migrate empty changelog");
    }

    #[test]
    fn migrations_multiple() {
        let mut conn = connect();
        let changelog1 = [("20220101", "CREATE TABLE one (id INT);")];
        let changelog2 = [
            ("20220101", "CREATE TABLE one (id INT);"),
            ("20220102", "CREATE TABLE two (text TEXT);"),
        ];
        let changelog3 = [
            ("20220101", "CREATE TABLE one (id INT);"),
            ("20220102", "CREATE TABLE two (text TEXT);"),
            ("20220103", "CREATE TABLE three (text TEXT);"),
            ("20220104", "CREATE TABLE four (text TEXT);"),
        ];

        apply(&mut conn, &changelog1).expect("Migrate changelog1");
        apply(&mut conn, &changelog2).expect("Migrate changelog2");
        apply(&mut conn, &changelog3).expect("Migrate changelog3");

        let inserted = conn
            .execute("INSERT INTO four (text) VALUES (?)", params!["test"])
            .expect("Insert into table");
        assert_eq!(inserted, 1);

        assert_db_changelog(
            &mut conn,
            vec![
                (
                    "20220101".to_string(),
                    "7501be37c8dac3cb135ff044ba0f126ceaa135f010cf0749edf23a64153ee736".to_string(),
                ),
                (
                    "20220102".to_string(),
                    "654c14bcb924f31c73679cfc01cf3152dc5afb8634f7a63027f8271e62c1ee0a".to_string(),
                ),
                (
                    "20220103".to_string(),
                    "c2dc6f389507e24707403abdc7eb32e680a0cba1809f845f0c7d34850de01ca4".to_string(),
                ),
                (
                    "20220104".to_string(),
                    "0332d0dde4c3dff8cabcbceb1423dd0daf195a012a55689f9a410054399ce7ec".to_string(),
                ),
            ],
        );
    }

    #[test]
    fn migration_mismatch() {
        let mut conn = connect();
        let changelog1 = [
            ("20220101", "CREATE TABLE one (id INT);"),
            ("20220102", "CREATE TABLE two (text TEXT);"),
        ];
        let changelog2 = [
            ("20220101", "CREATE TABLE one (id INT);"),
            ("20220102", "CREATE TABLE three (text TEXT);"),
        ];

        apply(&mut conn, &changelog1).expect("Migrate changelog1");

        let res = apply(&mut conn, &changelog2);
        match res {
            Err(MigrationError::HashMismatch {
                version,
                expected,
                actual,
            }) => {
                assert_eq!(version, "20220102");
                assert_eq!(
                    expected,
                    "c2dc6f389507e24707403abdc7eb32e680a0cba1809f845f0c7d34850de01ca4"
                );
                assert_eq!(
                    actual,
                    "654c14bcb924f31c73679cfc01cf3152dc5afb8634f7a63027f8271e62c1ee0a"
                );
            }
            _ => {
                panic!(
                    "Expected Err(MigrationError::HashMismatch {{ .. }}) received {:?}",
                    res
                );
            }
        }

        assert_db_changelog(
            &mut conn,
            vec![
                (
                    "20220101".to_string(),
                    "7501be37c8dac3cb135ff044ba0f126ceaa135f010cf0749edf23a64153ee736".to_string(),
                ),
                (
                    "20220102".to_string(),
                    "654c14bcb924f31c73679cfc01cf3152dc5afb8634f7a63027f8271e62c1ee0a".to_string(),
                ),
            ],
        );
    }

    #[test]
    fn test_seahash_scalar() {
        // Establish connection
        let file_name = format!("file:test-{}?mode=memory", rand::random::<u16>());
        let conn = Connection::open(file_name).unwrap();
        add_seahash(&conn).unwrap();

        // Create a table
        conn.execute("CREATE TABLE names (name TEXT)", []).unwrap();

        // Populate with data
        conn.execute("INSERT INTO names (name) VALUES (?)", ["Tom"])
            .unwrap();

        // Calculate a hash
        let hash: String = conn
            .query_row("SELECT seahash(name) FROM names", [], |row| row.get(0))
            .unwrap();

        let mut hasher = SeaHasher::new();
        hasher.write(b"Tom");
        let expected_hash = hasher.finish().to_string();
        assert_eq!(hash, expected_hash);
        assert_eq!(hash, "9975570841359408637");
    }

    #[test]
    fn test_seahash_aggregate() {
        // Establish connection
        let file_name = format!("file:test-{}?mode=memory", rand::random::<u16>());
        let conn = Connection::open(file_name).unwrap();
        add_group_seahash(&conn).unwrap();

        // Create a table
        conn.execute(
            r#"
CREATE TABLE names (
  name TEXT,
  age INT
);"#,
            [],
        )
        .unwrap();

        // Populate with data
        let rows = &[
            ("Tom", 10),
            ("Sam", 16),
            ("Alice", 18),
            ("Bob", 20),
            ("Mary", 24),
        ];
        for (name, age) in rows {
            conn.execute(
                "INSERT INTO names (name, age) VALUES (?, ?)",
                params![name, age],
            )
            .unwrap();
        }

        // Agreggate hashes
        let (hash, names): (String, String) = conn.query_row(
            "SELECT group_seahash(name), group_concat(name) FROM (SELECT name FROM names WHERE age < 17)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).unwrap();
        assert_eq!(names, "Tom,Sam");
        assert_eq!(hash, "6930023242532742803");

        let (hash, names): (String, String) = conn
            .query_row(
                "SELECT group_seahash(name), group_concat(name) FROM (SELECT name FROM names WHERE age > 17)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(names, "Alice,Bob,Mary");
        assert_eq!(hash, "14179263027409634498");

        // Verify empty rows
        let (hash, names): (Option<String>, Option<String>) = conn.query_row(
            "SELECT group_seahash(name), group_concat(name) FROM (SELECT name FROM names WHERE age < 5)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).unwrap();
        assert!(names.is_none());
        assert!(hash.is_none());
    }
}
