-- 006_kursi_sesi_keamanan.sql
--
-- A. KURSI BERNOMOR — denah dibangkitkan dari kapasitas + konfigurasi (lihat
--    `web/seats.rs`, dipakai server & UI). Tiap kursi terjual = satu baris
--    order_seats; PRIMARY KEY (schedule_id, kode) yang mencegah dua order
--    memegang kursi yang sama walau checkout serentak.
--    Order lama (sebelum migrasi ini) tak punya nomor kursi — mereka tetap
--    dihitung di `schedules.kursi_terjual`, jadi kapasitas tetap terjaga.
ALTER TABLE schedules
    ADD COLUMN IF NOT EXISTS konfigurasi  TEXT    NOT NULL DEFAULT '2-2'
        CHECK (konfigurasi IN ('2-2', '2-1', '1-1')),
    ADD COLUMN IF NOT EXISTS dua_dek      BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS kursi_wanita TEXT[]  NOT NULL DEFAULT '{}';

CREATE TABLE IF NOT EXISTS order_seats (
    schedule_id UUID NOT NULL REFERENCES schedules(id) ON DELETE CASCADE,
    kode        TEXT NOT NULL,
    order_id    UUID NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    PRIMARY KEY (schedule_id, kode)
);
CREATE INDEX IF NOT EXISTS idx_order_seats_order ON order_seats (order_id);

-- B. PERANGKAT & SESI — satu baris per login. Token sesi membawa id baris ini
--    (`user|role|iat|sid`); sesi yang dicabut dimuat ke memori saat start
--    sehingga verifikasi cookie tetap tanpa query DB per request.
CREATE TABLE IF NOT EXISTS user_sessions (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    user_agent    TEXT        NOT NULL DEFAULT '',
    ip            TEXT        NOT NULL DEFAULT '',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at    TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_user_sessions_user ON user_sessions (user_id, last_seen_at DESC);

-- C. RIWAYAT AKTIVITAS KEAMANAN
CREATE TABLE IF NOT EXISTS security_events (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    jenis       TEXT        NOT NULL CHECK (jenis IN (
                    'login_berhasil', 'login_gagal', 'sandi_diubah', 'lupa_sandi',
                    'nomor_diubah', 'sesi_dicabut', 'keluar_semua',
                    'akun_dibekukan', 'akun_dipulihkan')),
    ip          TEXT        NOT NULL DEFAULT '',
    user_agent  TEXT        NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_security_events_user ON security_events (user_id, created_at DESC);

-- D. BEKUKAN AKUN — login dengan password biasa ditolak; dibuka kembali
--    lewat "Lupa Password" (password baru dikirim ke WhatsApp terdaftar).
ALTER TABLE users ADD COLUMN IF NOT EXISTS frozen_at TIMESTAMPTZ;
