# syntax=docker/dockerfile:1.7
# ═══════════════════════════════════════════════════════════════════════════════
# Dockerfile — LajuBus / bis (Leptos SSR + Axum, satu binary satu port)
#
#   Stage 1 (builder): rust:1.95-alpine (musl, STATIS)
#     a. cargo-leptos DIPIN — layer sendiri, rerun hanya saat toolchain berubah
#     b. pre-compile dependency dengan dummy src — cache s/d Cargo.toml/lock berubah
#     c. cargo leptos build --release --precompress → WASM + SSR + .br/.gz sekali jalan
#   Stage 2 (runtime): debian:bookworm-slim, NON-ROOT
#
# Pola sama dengan e-ticketing & ppm. TIDAK ADA TEST DI SINI — disengaja: test
# milik job CI terpisah, supaya kegagalan test tak tercampur log build Docker.
#
# Env WAJIB saat runtime:
#   DATABASE_URL, SESSION_SECRET (acak & panjang — menandatangani semua cookie
#   sesi), ADMIN_PHONE, ADMIN_PASSWORD
#
# Opsional: AUTO_MIGRATE (default true), ADMIN_NAME, WAHA_BASE_URL/SESSION/API_KEY,
#   RUSTFS_ENDPOINT/ACCESS_KEY/SECRET_KEY/BUCKET/PUBLIC_URL, RUST_LOG
#
# Port: app mendengar di 3000 DI DALAM container (SITE_ADDR di bawah). Di host
# produksi dipetakan ke 3400 — pingora-kinetic meneruskan lajubus.online ke
# 127.0.0.1:3400. (3000 panel WA, 3100 e-ticketing, 3200-3202 ppm, 3300 Gitea.)
#
# Run:  docker build -t bis .
#       docker run -p 3400:3000 --env-file .env bis
# ═══════════════════════════════════════════════════════════════════════════════

# ── Builder ───────────────────────────────────────────────────────────────────
# Versi DIPIN. MSRV tertinggi di Cargo.lock saat ini 1.94.1 (crate aws-smithy-*);
# naikkan tag ini bila `cargo update` menarik dependency yang menuntut lebih baru.
FROM rust:1.95-alpine AS builder

# cmake + linux-headers: jaring pengaman untuk aws-lc-sys (rustls/aws-sdk-s3) —
# versi sekarang memakai `cc`, tapi fallback-nya butuh cmake. binaryen = wasm-opt,
# brotli dipakai --precompress.
RUN apk add --no-cache \
    musl-dev g++ make perl pkgconfig cmake linux-headers \
    openssl-dev openssl-libs-static \
    zlib-dev zlib-static \
    curl binaryen brotli

RUN rustup target add wasm32-unknown-unknown

# cargo-leptos sebagai layer sendiri, VERSI DIPIN (= `cargo leptos --version` di
# laptop pengembang). `--locked` hanya mengunci dependensinya, bukan versinya —
# tanpa `--version`, rilis baru bisa mengubah tata letak keluaran /pkg diam-diam.
#
# wasm-bindgen: cargo-leptos 0.3.9 membaca versi crate dari Cargo.lock (=0.2.122,
# dipin di Cargo.toml) lalu mengunduh CLI yang PERSIS sama — tak perlu memasang
# wasm-bindgen-cli sendiri, dan skema bindgen dijamin cocok.
RUN --mount=type=cache,id=bis-cargo-registry,target=/usr/local/cargo/registry \
    cargo install cargo-leptos --locked --version 0.3.9

# Versi Tailwind dipatri di biner cargo-leptos dan diunduh saat build. Env ini
# mengunci compiler CSS supaya produksi = lokal (v4.2.1, lihat cache cargo-leptos).
ENV LEPTOS_TAILWIND_VERSION=v4.2.1

ENV OPENSSL_STATIC=1
ENV PKG_CONFIG_ALLOW_CROSS=1
WORKDIR /app

# ── Pre-compile dependency ────────────────────────────────────────────────────
# Salin HANYA input yang memengaruhi dependency → layer ini invalid saat
# Cargo.toml/lock/build.rs/migration/style berubah, BUKAN saat edit src/.
COPY Cargo.toml Cargo.lock build.rs ./
# migration/ WAJIB ada sebelum compile apa pun: build.rs meng-embed setiap
# migration/*.sql ke binari (`fs::read_dir("migration").expect(...)`) — tanpa
# ini build.rs panic. Pelajaran yang sama dari e-ticketing.
COPY migration/ ./migration/
# style/ = `tailwind-input-file`, public/ = `assets-dir` (cargo-leptos gagal bila
# tak ada). public/ memuat ikon app + manifest yang disajikan dari root situs.
COPY style/ ./style/
COPY public/ ./public/

# Dummy source agar Cargo meng-compile & men-cache SELURUH dependency.
RUN mkdir -p src && \
    printf 'fn main() {}' > src/main.rs && \
    printf '' > src/lib.rs

# Dua langkah di bawah HANYA memanaskan cache — `|| true` disengaja. Build yang
# sesungguhnya (tanpa jaring pengaman) ada di bawah.
#
# Deps SSR: fitur persis seperti yang dipakai cargo-leptos untuk binari
# (`--no-default-features --features=ssr`), profil release.
RUN --mount=type=cache,id=bis-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=bis-target,target=/app/target \
    cargo build --release --no-default-features --features ssr 2>&1 || true

# Deps WASM: `--profile wasm-release` (= lib-profile-release di Cargo.toml) dan
# `--no-default-features` (default = ssr; ssr+hydrate bersamaan tidak sah).
# Profil/target berbeda = direktori artefak berbeda — meniru persis perintah
# cargo-leptos adalah satu-satunya cara cache ini benar-benar terpakai.
RUN --mount=type=cache,id=bis-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=bis-target,target=/app/target \
    cargo build --profile wasm-release --target wasm32-unknown-unknown \
    --no-default-features --features hydrate --lib 2>&1 || true

# ── Build final ───────────────────────────────────────────────────────────────
COPY src/ ./src/
# Sentuh agar Cargo tahu source berubah setelah swap dummy→real.
RUN touch src/main.rs src/lib.rs

# `--precompress`: .br + .gz untuk tiap aset /pkg dibuat SEKALI di sini dan
# disajikan `ServeDir::precompressed_br()` (main.rs) — bundle WASM tak dikompresi
# ulang per klien di VPS kecil.
#
# Salinan `bis_bg.wasm`: glue JS hasil wasm-bindgen memakai `bis_bg.wasm` sebagai
# nama bawaan, sedangkan cargo-leptos menulis `bis.wasm`. Server meminta
# `bis.wasm` (LEPTOS_OUTPUT_NAME di-set saat compile), jadi ini jaring pengaman
# murah: kalau suatu saat nama itu bergeser, hydration tetap jalan alih-alih
# 404 diam-diam yang membuat semua tombol mati (pernah menimpa e-ticketing).
RUN --mount=type=cache,id=bis-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=bis-target,target=/app/target \
    cargo leptos build --release --precompress \
    && cp /app/target/release/bis /app/bis-bin \
    && cp -r /app/target/site /app/site-out \
    && cd /app/site-out/pkg \
    && for ext in "" .br .gz; do \
         if [ -f "bis.wasm${ext}" ] && [ ! -f "bis_bg.wasm${ext}" ]; then \
           cp "bis.wasm${ext}" "bis_bg.wasm${ext}"; \
         fi; \
       done \
    && test -f bis.wasm && test -f bis.js && test -f bis.css \
    && ls -la /app/site-out/pkg/

# ── Runtime ───────────────────────────────────────────────────────────────────
# Binari musl STATIS (OPENSSL_STATIC=1) → jalan di distro mana pun. Debian dipilih
# karena curl (HEALTHCHECK) + ca-certificates (HTTPS ke WAHA/RustFS) teruji.
# JANGAN ganti builder ke target glibc sambil membiarkan runtime ini apa adanya.
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

# NON-ROOT: proses memegang kredensial Postgres, RustFS, WAHA, dan kunci sesi.
# UID tetap (10001) supaya kepemilikan volume ter-mount stabil antar rebuild.
RUN useradd --system --uid 10001 --create-home --shell /usr/sbin/nologin lajubus

WORKDIR /app

COPY --from=builder /app/bis-bin   ./bis
COPY --from=builder /app/site-out  ./target/site
# Cargo.toml WAJIB saat runtime: `get_configuration(Some("Cargo.toml"))` membaca
# [package.metadata.leptos]. Hilang → panic saat start.
COPY --from=builder /app/Cargo.toml ./Cargo.toml

RUN chown -R lajubus:lajubus /app
USER lajubus

# SITE_ADDR dibaca config.rs (default kode 0.0.0.0:3100 — itu untuk dev lokal).
# Di container selalu 3000; pemetaan port host diatur `docker run -p`.
ENV SITE_ADDR=0.0.0.0:3000
ENV LEPTOS_SITE_ADDR=0.0.0.0:3000
ENV LEPTOS_SITE_ROOT=target/site
ENV LEPTOS_ENV=PROD
ENV RUST_LOG=info

EXPOSE 3000

# /healthz murah (tanpa query DB). start-period 45s: startup menyambung Postgres,
# menjalankan migrasi (AUTO_MIGRATE) di bawah advisory lock, men-seed admin, dan
# memeriksa bucket RustFS (timeout 3 dtk) — jangan sampai di-restart sebelum siap.
HEALTHCHECK --interval=15s --timeout=3s --start-period=45s --retries=3 \
    CMD curl -fsS http://localhost:3000/healthz || exit 1

CMD ["./bis"]
