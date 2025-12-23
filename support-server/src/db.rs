use rusqlite::{Connection, Result};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

pub type DbPool = Arc<Mutex<Connection>>;

// Minimal valid empty ZIP file (22 bytes)
const EMPTY_ZIP: &[u8] = &[
    0x50, 0x4B, 0x05, 0x06, // End of central directory signature
    0x00, 0x00, // Number of this disk
    0x00, 0x00, // Disk where central directory starts
    0x00, 0x00, // Number of central directory records on this disk
    0x00, 0x00, // Total number of central directory records
    0x00, 0x00, 0x00, 0x00, // Size of central directory
    0x00, 0x00, 0x00, 0x00, // Offset of start of central directory
    0x00, 0x00, // Comment length
];

pub fn init_db(path: &str, encryption_key: &str) -> Result<DbPool> {
    let conn = Connection::open(path)?;

    // Set SQLCipher encryption key
    conn.execute_batch(&format!("PRAGMA key = '{}';", encryption_key))?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS tickets (
            id INTEGER PRIMARY KEY,
            user_id INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            description TEXT NOT NULL,
            zip_data BLOB NOT NULL,
            zip_filename TEXT NOT NULL,
            state TEXT NOT NULL DEFAULT 'new'
        );

        CREATE TABLE IF NOT EXISTS comments (
            id INTEGER PRIMARY KEY,
            ticket_id INTEGER NOT NULL,
            user_id INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            text TEXT NOT NULL,
            FOREIGN KEY (ticket_id) REFERENCES tickets(id)
        );

        CREATE INDEX IF NOT EXISTS idx_tickets_user_id ON tickets(user_id);
        CREATE INDEX IF NOT EXISTS idx_comments_ticket_id ON comments(ticket_id);
        ",
    )?;

    Ok(Arc::new(Mutex::new(conn)))
}

pub fn seed_db(pool: &DbPool) -> Result<()> {
    let conn = pool.lock().unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Check if tickets already exist
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM tickets", [], |row| row.get(0))?;
    if count > 0 {
        println!("Seed data: Tickets already exist, skipping ticket seeding");
        return Ok(());
    }

    // Seed dummy tickets
    conn.execute(
        "INSERT INTO tickets (user_id, created_at, description, zip_data, zip_filename, state)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        (
            1,
            now - 3600, // 1 hour ago
            "App stürzt beim Start ab. Logs im Anhang.",
            EMPTY_ZIP,
            "crash_logs.zip",
            "new",
        ),
    )?;

    conn.execute(
        "INSERT INTO tickets (user_id, created_at, description, zip_data, zip_filename, state)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        (
            1,
            now - 86400, // 1 day ago
            "Synchronisation funktioniert nicht mehr seit dem letzten Update.",
            EMPTY_ZIP,
            "sync_debug.zip",
            "in_progress",
        ),
    )?;

    // Add a comment to the second ticket
    conn.execute(
        "INSERT INTO comments (ticket_id, user_id, created_at, text) VALUES (?1, ?2, ?3, ?4)",
        (
            2,
            100, // admin user
            now - 43200, // 12 hours ago
            "Wir schauen uns das Problem an. Könnten Sie bitte noch die App-Version angeben?",
        ),
    )?;

    println!("Seed data created: 2 dummy tickets with 1 comment");
    println!("Note: API keys are now managed via identity-server");
    Ok(())
}
