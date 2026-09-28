//! config/config.rs — konfigurasi aplikasi dari environment variable.

#[derive(Clone)]
pub struct RustFsConfig {
    pub endpoint: String,
    pub access_key: String,
    pub secret_key: String,
    pub bucket: String,
    /// Base public URL tanpa trailing slash.
    pub public_url: String,
}

#[derive(Clone)]
pub struct WahaConfig {
    /// Tanpa trailing slash, mis. "http://localhost:3000".
    pub base_url: String,
    pub session: String,
    /// Header `X-Api-Key` — kosong berarti tak dikirim (WAHA tanpa auth).
    pub api_key: String,
}

#[derive(Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub auto_migrate: bool,
    /// Dipakai men-seed SATU akun admin saat startup (lihat `main.rs`),
    /// bukan lagi password cookie terpisah — admin login lewat mekanisme
    /// sesi yang sama dengan buyer/merchant (no. HP + password).
    pub admin_password: String,
    pub admin_phone: String,
    pub admin_name: String,
    /// Kunci tanda tangan SEMUA cookie sesi (buyer/merchant/admin).
    pub session_secret: String,
    pub site_addr: String,
    pub rustfs: RustFsConfig,
    pub waha: WahaConfig,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| anyhow::anyhow!("DATABASE_URL wajib di-set"))?;
        let auto_migrate = std::env::var("AUTO_MIGRATE")
            .map(|v| v != "false")
            .unwrap_or(true);
        let admin_password =
            std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "nyentrix123".to_string());
        let admin_phone =
            std::env::var("ADMIN_PHONE").unwrap_or_else(|_| "620000000000".to_string());
        let admin_name =
            std::env::var("ADMIN_NAME").unwrap_or_else(|_| "Admin PO Nyentrix Trans".to_string());
        let session_secret = std::env::var("SESSION_SECRET")
            .or_else(|_| std::env::var("ADMIN_SESSION_SECRET"))
            .unwrap_or_else(|_| "dev-only-change-me".to_string());
        let site_addr = std::env::var("SITE_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());

        let rustfs = RustFsConfig {
            endpoint: std::env::var("RUSTFS_ENDPOINT")
                .unwrap_or_else(|_| "http://127.0.0.1:9000".to_string()),
            access_key: std::env::var("RUSTFS_ACCESS_KEY").unwrap_or_default(),
            secret_key: std::env::var("RUSTFS_SECRET_KEY").unwrap_or_default(),
            bucket: std::env::var("RUSTFS_BUCKET").unwrap_or_else(|_| "bis".to_string()),
            public_url: std::env::var("RUSTFS_PUBLIC_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:9000/bis".to_string()),
        };

        let waha = WahaConfig {
            base_url: std::env::var("WAHA_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:3000".to_string())
                .trim_end_matches('/')
                .to_string(),
            session: std::env::var("WAHA_SESSION").unwrap_or_else(|_| "default".to_string()),
            api_key: std::env::var("WAHA_API_KEY").unwrap_or_default(),
        };

        Ok(Self {
            database_url,
            auto_migrate,
            admin_password,
            admin_phone,
            admin_name,
            session_secret,
            site_addr,
            rustfs,
            waha,
        })
    }
}
