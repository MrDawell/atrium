use rusqlite::Connection;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct MemoryStore {
    conn: Mutex<Connection>,
}

impl MemoryStore {
    pub fn new(db_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let conn = if db_path == ":memory:" {
            Connection::open_in_memory()?
        } else {
            Connection::open(db_path)?
        };
        let store = Self { conn: Mutex::new(conn) };
        store.initialize_schema()?;
        Ok(store)
    }

    fn initialize_schema(&self) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| format!("Mutex lock error: {}", e))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS symbols (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_path TEXT NOT NULL,
                symbol_name TEXT NOT NULL,
                symbol_kind TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                UNIQUE(file_path, symbol_name)
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS durable_memories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                fact TEXT NOT NULL,
                scope TEXT NOT NULL,
                confidence REAL NOT NULL,
                last_validated_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Speed up scope matching queries
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_durable_memories_scope ON durable_memories(scope)",
            [],
        )?;

        Ok(())
    }

    pub fn store_symbols(
        &self,
        file_path: &str,
        hash: &str,
        symbols: &[atrium_core_graph::SymbolNode],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut conn = self.conn.lock().map_err(|e| format!("Mutex lock error: {}", e))?;
        let tx = conn.transaction()?;

        // 1. Clear stale symbols for this file path
        tx.execute("DELETE FROM symbols WHERE file_path = ?1", [file_path])?;

        // 2. Insert newly parsed symbols
        let mut stmt = tx.prepare(
            "INSERT OR REPLACE INTO symbols (file_path, symbol_name, symbol_kind, content_hash)
             VALUES (?1, ?2, ?3, ?4)",
        )?;

        for sym in symbols {
            stmt.execute([file_path, &sym.name, &sym.kind, hash])?;
        }

        drop(stmt);
        tx.commit()?;
        Ok(())
    }

    pub fn add_durable_fact(
        &self,
        fact: &str,
        scope: &str,
        confidence: f64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| format!("Mutex lock error: {}", e))?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        conn.execute(
            "INSERT INTO durable_memories (fact, scope, confidence, last_validated_at)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![fact, scope, confidence, now],
        )?;
        Ok(())
    }

    pub fn get_relevant_facts(
        &self,
        file_path: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| format!("Mutex lock error: {}", e))?;
        let file_scope = format!("file:{}", file_path);

        let mut stmt = conn.prepare(
            "SELECT fact FROM durable_memories
             WHERE scope = 'global' OR scope = ?1 OR scope = ?2",
        )?;

        let rows = stmt.query_map([file_path, &file_scope], |row| row.get(0))?;

        let mut facts = Vec::new();
        for fact_res in rows {
            facts.push(fact_res?);
        }

        Ok(facts)
    }

    pub fn get_symbols_for_file(
        &self,
        file_path: &str,
    ) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| format!("Mutex lock error: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT symbol_name, symbol_kind FROM symbols WHERE file_path = ?1",
        )?;

        let rows = stmt.query_map([file_path], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut symbols = Vec::new();
        for sym_res in rows {
            symbols.push(sym_res?);
        }

        Ok(symbols)
    }

    pub fn get_indexed_files(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| format!("Mutex lock error: {}", e))?;
        let mut stmt = conn.prepare("SELECT DISTINCT file_path FROM symbols")?;
        let rows = stmt.query_map([], |row| row.get(0))?;

        let mut files = Vec::new();
        for file_res in rows {
            files.push(file_res?);
        }

        Ok(files)
    }
}
