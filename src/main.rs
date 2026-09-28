//! main.rs — entry point bis (Leptos SSR + Axum), satu binary satu port.
//!
//!   /            → Leptos SSR (dashboard satu halaman, 4 tab)
//!   /api-fn/*    → Leptos server function
//!   /upload/*    → upload foto trip (multipart)
//!   /pkg/*       → aset statis (WASM/JS/CSS)

// Tipe view Leptos (mis. BrowsePage) sangat dalam; build RELEASE bin ini
// menghitung layout future-nya dan melewati batas bawaan (128) → "queries
// overflow the depth limit". Build dev tak terkena, jadi baru ketahuan di
// `cargo leptos build --release` / Docker. Samakan dengan lib.rs.
#![recursion_limit = "512"]

use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use bis::config::{config::AppConfig, database::create_pool, migrate};
use bis::repository::{
    ArmadaRepository, BannerRepository, OrderRepository, OtpRepository, PhoneChangeRepository, RatingRepository, RentalRepository, RouteRepository, ScheduleRepository, TripPhotoRepository,
    UserRepository,
};
use bis::service::{
    auth::ensure_admin_seed, ArmadaService, AuthService, BannerService, OrderService, PhotoService, RateLimiter, RatingService, RentalService,
    RouteService, ScheduleService, StorageService, WahaClient,
};
use bis::state::AppState;
use bis::web::api::upload::{listing_photo_upload, trip_photo_upload};
use bis::web::app::{shell, App};

use leptos::config::get_configuration;
use leptos_axum::{generate_route_list, LeptosRoutes};

#[tokio::main]
async fn main() -> Result<()> {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("gagal memasang rustls crypto provider");

    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "bis=info,tower_http=info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cfg = AppConfig::from_env()?;

    let pool = create_pool(&cfg.database_url, 10).await.context("gagal membuat pool Postgres")?;
    tracing::info!("Postgres pool siap");

    if cfg.auto_migrate {
        migrate::run(&pool).await.context("migrasi database gagal")?;
    } else {
        tracing::warn!("AUTO_MIGRATE dimatikan — skema tidak diperiksa");
    }

    // RustFS opsional saat start: endpoint yang tak terjangkau (mis. dev lokal
    // tanpa akses ke bridge Docker VPS) sebelumnya menahan startup belasan
    // detik menunggu `dispatch failure`. Timeout pendek + lanjut jalan — upload
    // foto trip akan gagal sampai endpoint benar, tapi sisa aplikasi tetap hidup.
    let storage = StorageService::new(&cfg.rustfs);
    match tokio::time::timeout(std::time::Duration::from_secs(3), storage.init()).await {
        Ok(_) => {}
        Err(_) => tracing::warn!(
            endpoint = %cfg.rustfs.endpoint,
            "RustFS: timeout memeriksa bucket saat start — lanjut jalan, upload akan gagal sampai endpoint terjangkau"
        ),
    }

    let user_repo = UserRepository::new(pool.clone());

    ensure_admin_seed(&user_repo, &cfg.admin_phone, &cfg.admin_name, &cfg.admin_password)
        .await
        .context("gagal men-seed akun admin")?;

    let armada_repo = ArmadaRepository::new(pool.clone());
    let waha = WahaClient::new(&cfg.waha);
    let rate_limiter = Arc::new(RateLimiter::new());

    let state = Arc::new(AppState {
        pool: pool.clone(),
        rate: rate_limiter.clone(),
        armada_svc: ArmadaService::new(armada_repo.clone()),
        schedule_svc: ScheduleService::new(ScheduleRepository::new(pool.clone()), armada_repo.clone()),
        route_svc: RouteService::new(RouteRepository::new(pool.clone()), armada_repo),
        photo_svc: PhotoService::new(TripPhotoRepository::new(pool.clone())),
        order_svc: OrderService::new(OrderRepository::new(pool.clone())),
        rating_svc: RatingService::new(RatingRepository::new(pool.clone()), OrderRepository::new(pool.clone())),
        rental_svc: RentalService::new(
            RentalRepository::new(pool.clone()),
            waha.clone(),
            rate_limiter.clone(),
            bis::utils::phone::normalize(&cfg.admin_phone).unwrap_or_default(),
        ),
        banner_svc: BannerService::new(BannerRepository::new(pool.clone())),
        auth_svc: AuthService::new(
            user_repo,
            OtpRepository::new(pool.clone()),
            PhoneChangeRepository::new(pool.clone()),
            bis::repository::security::SecurityRepository::new(pool.clone()),
            waha,
            rate_limiter,
            cfg.session_secret.clone(),
        ),
        storage,
    });

    // Trayek tetap → jadwal harian 30 hari ke depan (sekarang, lalu tiap jam).
    state.route_svc.spawn_generator();

    // Sesudah migrasi 005: cookie sesi yang sudah dicabut pemiliknya
    // ("keluarkan dari semua perangkat lain") harus ditolak sejak request pertama.
    state
        .auth_svc
        .load_session_revocations()
        .await
        .context("gagal memuat pencabutan sesi")?;

    let leptos_conf = get_configuration(Some("Cargo.toml"))
        .map_err(|e| anyhow::anyhow!("gagal memuat konfigurasi leptos: {e}"))?;
    let leptos_options = leptos_conf.leptos_options;
    let site_root = leptos_options.site_root.to_string();

    let socket_addr: std::net::SocketAddr = cfg
        .site_addr
        .parse()
        .map_err(|e| anyhow::anyhow!("SITE_ADDR tidak valid `{}`: {e}", cfg.site_addr))?;

    let ssr_routes = generate_route_list(App);

    let leptos_router = axum::Router::new()
        .leptos_routes(&leptos_options, ssr_routes, {
            let opts = leptos_options.clone();
            move || shell(opts.clone())
        })
        // `.br`/`.gz` dibuat SEKALI oleh `cargo leptos build --precompress` di
        // Dockerfile — bundle WASM tak perlu dikompresi ulang per klien. Tanpa
        // berkas itu (dev lokal) ServeDir menyajikan yang asli dan
        // CompressionLayer di bawah mengambil alih seperti biasa.
        .route_service(
            "/pkg/{*path}",
            tower_http::services::ServeDir::new(&site_root)
                .precompressed_br()
                .precompressed_gzip(),
        )
        .fallback(leptos_axum::file_and_error_handler(shell))
        .layer(axum::middleware::from_fn_with_state(state.clone(), bis::middleware::auth::page_guard))
        .layer(axum::Extension(state.clone()))
        .with_state(leptos_options);

    let upload_router = axum::Router::new()
        .route("/upload/trip-photo", axum::routing::post(trip_photo_upload))
        .route("/upload/listing-photo", axum::routing::post(listing_photo_upload))
        .layer(axum::extract::DefaultBodyLimit::max(8 * 1024 * 1024))
        .layer(axum::Extension(state.clone()));

    let health_router = axum::Router::new().route("/healthz", axum::routing::get(|| async { "ok" }));

    let app = health_router
        .merge(upload_router)
        .merge(leptos_router)
        .layer(tower_http::compression::CompressionLayer::new());

    let listener = TcpListener::bind(socket_addr)
        .await
        .with_context(|| format!("gagal mengikat {socket_addr}"))?;
    tracing::info!("bis listening on http://{socket_addr}");

    axum::serve(listener, app).await?;

    Ok(())
}
