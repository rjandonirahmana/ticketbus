//! repository/security.rs — sesi login per perangkat (`user_sessions`) dan
//! riwayat aktivitas keamanan (`security_events`). Lihat migrasi 006.

use chrono::{DateTime, Utc};
use deadpool_postgres::Pool;
use uuid::Uuid;

/// Satu baris sesi mentah (belum diformat untuk UI).
pub struct SessionRow {
    pub id: String,
    pub user_agent: String,
    pub ip: String,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

pub struct EventRow {
    pub jenis: String,
    pub user_agent: String,
    pub ip: String,
    pub created_at: DateTime<Utc>,
}

/// Sesi dianggap mati bila tak terlihat selama ini (sama dengan umur cookie).
const SESSION_DAYS: i32 = 30;

#[derive(Clone)]
pub struct SecurityRepository {
    pool: Pool,
}

impl SecurityRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn create_session(&self, user_id: &str, ua: &str, ip: &str) -> anyhow::Result<String> {
        let uid = Uuid::parse_str(user_id)?;
        let conn = self.pool.get().await?;
        let row = conn
            .query_one(
                "INSERT INTO user_sessions (user_id, user_agent, ip) VALUES ($1, $2, $3) RETURNING id",
                &[&uid, &ua, &ip],
            )
            .await?;
        Ok(row.get::<_, Uuid>("id").to_string())
    }

    pub async fn touch_session(&self, id: &str) -> anyhow::Result<()> {
        let sid = Uuid::parse_str(id)?;
        let conn = self.pool.get().await?;
        conn.execute(
            "UPDATE user_sessions SET last_seen_at = NOW() WHERE id = $1 AND revoked_at IS NULL",
            &[&sid],
        )
        .await?;
        Ok(())
    }

    /// Sesi yang masih hidup: belum dicabut & terlihat dalam 30 hari terakhir.
    pub async fn active_sessions(&self, user_id: &str) -> anyhow::Result<Vec<SessionRow>> {
        let uid = Uuid::parse_str(user_id)?;
        let conn = self.pool.get().await?;
        let rows = conn
            .query(
                "SELECT id, user_agent, ip, created_at, last_seen_at FROM user_sessions
                  WHERE user_id = $1 AND revoked_at IS NULL
                    AND last_seen_at > NOW() - make_interval(days => $2)
                  ORDER BY last_seen_at DESC LIMIT 20",
                &[&uid, &SESSION_DAYS],
            )
            .await?;
        Ok(rows
            .iter()
            .map(|r| SessionRow {
                id: r.get::<_, Uuid>("id").to_string(),
                user_agent: r.get("user_agent"),
                ip: r.get("ip"),
                created_at: r.get("created_at"),
                last_seen_at: r.get("last_seen_at"),
            })
            .collect())
    }

    /// Cabut satu sesi milik `user_id`. `false` = tak ada (atau milik orang lain).
    pub async fn revoke_session(&self, user_id: &str, id: &str) -> anyhow::Result<bool> {
        let uid = Uuid::parse_str(user_id)?;
        let sid = Uuid::parse_str(id)?;
        let conn = self.pool.get().await?;
        let n = conn
            .execute(
                "UPDATE user_sessions SET revoked_at = NOW()
                  WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL",
                &[&sid, &uid],
            )
            .await?;
        Ok(n > 0)
    }

    /// Cabut semua sesi pengguna kecuali `kecuali` (None = semuanya).
    /// Mengembalikan id yang dicabut, untuk disalin ke daftar di memori.
    pub async fn revoke_sessions(&self, user_id: &str, kecuali: Option<&str>) -> anyhow::Result<Vec<String>> {
        let uid = Uuid::parse_str(user_id)?;
        let except = kecuali.map(Uuid::parse_str).transpose()?;
        let conn = self.pool.get().await?;
        let rows = conn
            .query(
                "UPDATE user_sessions SET revoked_at = NOW()
                  WHERE user_id = $1 AND revoked_at IS NULL AND ($2::uuid IS NULL OR id <> $2)
                  RETURNING id",
                &[&uid, &except],
            )
            .await?;
        Ok(rows.iter().map(|r| r.get::<_, Uuid>("id").to_string()).collect())
    }

    /// Sesi dicabut yang cookie-nya mungkin masih beredar (belum kedaluwarsa).
    pub async fn revoked_recent(&self) -> anyhow::Result<Vec<String>> {
        let conn = self.pool.get().await?;
        let rows = conn
            .query(
                "SELECT id FROM user_sessions
                  WHERE revoked_at IS NOT NULL AND created_at > NOW() - make_interval(days => $1)",
                &[&(SESSION_DAYS + 1)],
            )
            .await?;
        Ok(rows.iter().map(|r| r.get::<_, Uuid>("id").to_string()).collect())
    }

    pub async fn record(&self, user_id: &str, jenis: &str, ua: &str, ip: &str) -> anyhow::Result<()> {
        let uid = Uuid::parse_str(user_id)?;
        let conn = self.pool.get().await?;
        conn.execute(
            "INSERT INTO security_events (user_id, jenis, user_agent, ip) VALUES ($1, $2, $3, $4)",
            &[&uid, &jenis, &ua, &ip],
        )
        .await?;
        Ok(())
    }

    pub async fn recent_events(&self, user_id: &str, limit: i64) -> anyhow::Result<Vec<EventRow>> {
        let uid = Uuid::parse_str(user_id)?;
        let conn = self.pool.get().await?;
        let rows = conn
            .query(
                "SELECT jenis, user_agent, ip, created_at FROM security_events
                  WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2",
                &[&uid, &limit],
            )
            .await?;
        Ok(rows
            .iter()
            .map(|r| EventRow {
                jenis: r.get("jenis"),
                user_agent: r.get("user_agent"),
                ip: r.get("ip"),
                created_at: r.get("created_at"),
            })
            .collect())
    }

    pub async fn count_since(&self, user_id: &str, jenis: &str, days: i32) -> anyhow::Result<i64> {
        let uid = Uuid::parse_str(user_id)?;
        let conn = self.pool.get().await?;
        let row = conn
            .query_one(
                "SELECT COUNT(*) AS n FROM security_events
                  WHERE user_id = $1 AND jenis = $2 AND created_at > NOW() - make_interval(days => $3)",
                &[&uid, &jenis, &days],
            )
            .await?;
        Ok(row.get("n"))
    }

    pub async fn set_frozen(&self, user_id: &str, frozen: bool) -> anyhow::Result<()> {
        let uid = Uuid::parse_str(user_id)?;
        let conn = self.pool.get().await?;
        conn.execute(
            "UPDATE users SET frozen_at = CASE WHEN $2 THEN NOW() ELSE NULL END WHERE id = $1",
            &[&uid, &frozen],
        )
        .await?;
        Ok(())
    }

    pub async fn is_frozen(&self, user_id: &str) -> anyhow::Result<bool> {
        let uid = Uuid::parse_str(user_id)?;
        let conn = self.pool.get().await?;
        let row = conn
            .query_opt("SELECT frozen_at IS NOT NULL AS beku FROM users WHERE id = $1", &[&uid])
            .await?;
        Ok(row.map(|r| r.get("beku")).unwrap_or(false))
    }
}
