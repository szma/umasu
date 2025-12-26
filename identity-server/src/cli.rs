use std::time::{SystemTime, UNIX_EPOCH};

use crate::crypto::{generate_activation_code, generate_key};
use crate::db::DbPool;

fn now_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

pub fn create_user(db: &DbPool, email: &str, role: &str) -> Result<i64, String> {
    let conn = db.lock().unwrap();
    let now = now_timestamp();

    conn.execute(
        "INSERT INTO users (email, role, subscription_status, created_at) VALUES (?, ?, 'active', ?)",
        rusqlite::params![email, role, now],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();
    println!("Created user '{}' with id {}", email, id);
    Ok(id)
}

pub fn create_key(db: &DbPool, user_id: i64) -> Result<String, String> {
    let conn = db.lock().unwrap();

    // Verify user exists
    let email: String = conn
        .query_row("SELECT email FROM users WHERE id = ?", [user_id], |row| {
            row.get(0)
        })
        .map_err(|_| format!("User {} not found", user_id))?;

    let key = generate_key();
    let now = now_timestamp();

    conn.execute(
        "INSERT INTO api_keys (key_hash, key_prefix, user_id, created_at) VALUES (?, ?, ?, ?)",
        rusqlite::params![key.hash, key.prefix, user_id, now],
    )
    .map_err(|e| e.to_string())?;

    println!("==============================================");
    println!("API KEY CREATED (save this - shown only once!)");
    println!("Key:    {}", key.full_key);
    println!("Prefix: {}", key.prefix);
    println!("User:   {} (id={})", email, user_id);
    println!("==============================================");

    Ok(key.full_key)
}

pub fn revoke_key(db: &DbPool, prefix: &str) -> Result<(), String> {
    let conn = db.lock().unwrap();
    let now = now_timestamp();

    let rows = conn
        .execute(
            "UPDATE api_keys SET revoked_at = ? WHERE key_prefix = ? AND revoked_at IS NULL",
            rusqlite::params![now, prefix],
        )
        .map_err(|e| e.to_string())?;

    if rows == 0 {
        Err(format!("No active key found with prefix {}", prefix))
    } else {
        println!("Revoked key with prefix {}", prefix);
        Ok(())
    }
}

pub fn list_users(db: &DbPool) -> Result<(), String> {
    let conn = db.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, email, role, subscription_status, created_at FROM users ORDER BY id")
        .map_err(|e| e.to_string())?;

    let users = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    println!(
        "{:<5} {:<30} {:<10} {:<12} Created",
        "ID", "Email", "Role", "Status"
    );
    println!("{}", "-".repeat(75));

    for user in users {
        let (id, email, role, status, created) = user.map_err(|e| e.to_string())?;
        println!(
            "{:<5} {:<30} {:<10} {:<12} {}",
            id, email, role, status, created
        );
    }

    Ok(())
}

pub fn list_keys(db: &DbPool) -> Result<(), String> {
    let conn = db.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT k.id, k.key_prefix, u.email, k.created_at, k.revoked_at
             FROM api_keys k
             JOIN users u ON k.user_id = u.id
             ORDER BY k.id",
        )
        .map_err(|e| e.to_string())?;

    let keys = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, Option<i64>>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    println!(
        "{:<5} {:<15} {:<30} {:<12} Status",
        "ID", "Prefix", "User", "Created"
    );
    println!("{}", "-".repeat(75));

    for key in keys {
        let (id, prefix, email, created, revoked) = key.map_err(|e| e.to_string())?;
        let status = if revoked.is_some() {
            "revoked"
        } else {
            "active"
        };
        println!(
            "{:<5} {:<15} {:<30} {:<12} {}",
            id, prefix, email, created, status
        );
    }

    Ok(())
}

pub fn create_activation_code(db: &DbPool, user_id: i64) -> Result<String, String> {
    let conn = db.lock().unwrap();

    // Verify user exists
    let email: String = conn
        .query_row("SELECT email FROM users WHERE id = ?", [user_id], |row| {
            row.get(0)
        })
        .map_err(|_| format!("User {} not found", user_id))?;

    let code = generate_activation_code();
    let now = now_timestamp();

    conn.execute(
        "INSERT INTO activation_codes (code_hash, code_prefix, user_id, created_at) VALUES (?, ?, ?, ?)",
        rusqlite::params![code.hash, code.prefix, user_id, now],
    )
    .map_err(|e| e.to_string())?;

    println!("==============================================");
    println!("ACTIVATION CODE CREATED (shown only once!)");
    println!("Code:   {}", code.full_code);
    println!("Prefix: {}", code.prefix);
    println!("User:   {} (id={})", email, user_id);
    println!("==============================================");

    Ok(code.full_code)
}

pub fn list_activation_codes(db: &DbPool) -> Result<(), String> {
    let conn = db.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT a.id, a.code_prefix, u.email, a.created_at, a.used_at
             FROM activation_codes a
             JOIN users u ON a.user_id = u.id
             ORDER BY a.id",
        )
        .map_err(|e| e.to_string())?;

    let codes = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, Option<i64>>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    println!(
        "{:<5} {:<15} {:<30} {:<12} Status",
        "ID", "Prefix", "User", "Created"
    );
    println!("{}", "-".repeat(75));

    for code in codes {
        let (id, prefix, email, created, used) = code.map_err(|e| e.to_string())?;
        let status = if used.is_some() { "used" } else { "available" };
        println!(
            "{:<5} {:<15} {:<30} {:<12} {}",
            id, prefix, email, created, status
        );
    }

    Ok(())
}

pub fn seed_dev_data(db: &DbPool) -> Result<(), String> {
    println!("Seeding development data...\n");

    // Create dev users
    let admin_id = create_user(db, "admin@curadesk.local", "admin")?;
    let support_id = create_user(db, "support@curadesk.local", "support")?;
    let customer_id = create_user(db, "customer@curadesk.local", "customer")?;

    // Create keys for each
    println!("\n--- Admin Key ---");
    create_key(db, admin_id)?;

    println!("\n--- Support Key ---");
    create_key(db, support_id)?;

    println!("\n--- Customer Key ---");
    create_key(db, customer_id)?;

    // Create activation codes for testing
    println!("\n--- Customer Activation Code ---");
    create_activation_code(db, customer_id)?;

    println!("\nSeed data created successfully.");
    Ok(())
}
