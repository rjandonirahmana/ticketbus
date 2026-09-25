-- 002_marketplace.sql — 3 peran (buyer/merchant/admin), tiket per kursi,
-- order, dan rating armada+driver. Lihat CLAUDE.md / plan untuk alasan
-- desain (kenapa otp_pending di Postgres bukan Redis, dll).

CREATE TABLE IF NOT EXISTS users (
    id             UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    phone          TEXT        NOT NULL UNIQUE,
    name           TEXT        NOT NULL,
    role           TEXT        NOT NULL CHECK (role IN ('buyer', 'merchant', 'admin')),
    password_hash  TEXT        NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Draf pendaftaran sebelum OTP diverifikasi. Satu draf aktif per nomor HP —
-- daftar ulang menimpa draf lama (OTP/password sebelumnya otomatis basi).
CREATE TABLE IF NOT EXISTS otp_pending (
    phone          TEXT        PRIMARY KEY,
    name           TEXT        NOT NULL,
    role           TEXT        NOT NULL CHECK (role IN ('buyer', 'merchant')),
    password_hash  TEXT        NOT NULL,
    otp_code       TEXT        NOT NULL,
    attempts       INTEGER     NOT NULL DEFAULT 0,
    expires_at     TIMESTAMPTZ NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE armadas ADD COLUMN IF NOT EXISTS merchant_id UUID REFERENCES users(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_armadas_merchant_id ON armadas (merchant_id);

ALTER TABLE schedules
    ADD COLUMN IF NOT EXISTS kapasitas     INTEGER NOT NULL DEFAULT 40,
    ADD COLUMN IF NOT EXISTS kursi_terjual INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS driver_nama   TEXT    NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS driver_telp   TEXT    NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS orders (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    kode_order    TEXT        NOT NULL UNIQUE,
    buyer_id      UUID        NOT NULL REFERENCES users(id),
    schedule_id   UUID        NOT NULL REFERENCES schedules(id),
    jumlah_tiket  INTEGER     NOT NULL CHECK (jumlah_tiket > 0),
    harga_satuan  BIGINT      NOT NULL,
    total_harga   BIGINT      NOT NULL,
    nama_pemesan  TEXT        NOT NULL,
    telp_pemesan  TEXT        NOT NULL,
    status        TEXT        NOT NULL DEFAULT 'paid',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_orders_buyer_id ON orders (buyer_id);
CREATE INDEX IF NOT EXISTS idx_orders_schedule_id ON orders (schedule_id);

CREATE TABLE IF NOT EXISTS ratings (
    id             UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id       UUID        NOT NULL UNIQUE REFERENCES orders(id),
    buyer_id       UUID        NOT NULL REFERENCES users(id),
    armada_id      UUID        NOT NULL REFERENCES armadas(id),
    rating_armada  SMALLINT    NOT NULL CHECK (rating_armada BETWEEN 1 AND 5),
    rating_driver  SMALLINT    NOT NULL CHECK (rating_driver BETWEEN 1 AND 5),
    komentar       TEXT        NOT NULL DEFAULT '',
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_ratings_armada_id ON ratings (armada_id);
