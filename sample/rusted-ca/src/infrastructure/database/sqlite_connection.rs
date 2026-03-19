use rusqlite::{Connection, Result};
use std::sync::{Arc, Mutex};
use tokio::task;

#[derive(Clone)]
pub struct SqliteConnection {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteConnection {
    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open(":memory:")?;
        Self::run_migrations(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn run_migrations(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                name TEXT NOT NULL,
                password TEXT NOT NULL,
                phone TEXT,
                birth_date TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                last_login_at DATETIME
            )",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)",
            [],
        )?;
        println!("Database migrations completed successfully");
        Ok(())
    }

    // Command用メソッド（書き込み操作）
    pub async fn execute_command<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&mut Connection) -> Result<R> + Send + 'static,
        R: Send + 'static,
    {
        let conn = self.conn.clone();
        task::spawn_blocking(move || {
            let mut conn = conn.lock().unwrap();
            f(&mut *conn)
        })
        .await
        .unwrap()
    }

    // Query用メソッド（読み取り操作）
    pub async fn execute_query<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&mut Connection) -> Result<R> + Send + 'static,
        R: Send + 'static,
    {
        let conn = self.conn.clone();
        task::spawn_blocking(move || {
            let mut conn = conn.lock().unwrap();
            f(&mut *conn)
        })
        .await
        .unwrap()
    }
}
