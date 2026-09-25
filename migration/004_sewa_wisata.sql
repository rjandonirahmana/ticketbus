-- 004_sewa_wisata.sql — halaman "Sewa Bus Destinasi Wisata" (/wisata) dan
-- "Sewa Bus Rombongan & Korporat" (/sewa). Paket & katalog bus dikelola
-- admin (merchant_id NULL = milik platform) atau mitra PO (merchant_id =
-- pemiliknya). Permintaan dari penumpang disimpan di rental_requests dan
-- tampil di dashboard pemilik item (admin melihat semuanya).

CREATE TABLE IF NOT EXISTS tour_packages (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    merchant_id   UUID        REFERENCES users(id) ON DELETE CASCADE,
    judul         TEXT        NOT NULL,
    kategori      TEXT        NOT NULL DEFAULT 'alam'
                  CHECK (kategori IN ('alam', 'budaya', 'religi', 'edukasi', 'lainnya')),
    kawasan       TEXT        NOT NULL,
    durasi        TEXT        NOT NULL,
    label_badge   TEXT        NOT NULL DEFAULT '',
    label_tipe    TEXT        NOT NULL DEFAULT '',
    rute          TEXT        NOT NULL DEFAULT '',
    armada_info   TEXT        NOT NULL DEFAULT '',
    fasilitas     TEXT[]      NOT NULL DEFAULT '{}',
    harga_pax     BIGINT      NOT NULL DEFAULT 0 CHECK (harga_pax >= 0),
    min_pax       INTEGER     NOT NULL DEFAULT 1 CHECK (min_pax >= 1),
    harga_charter BIGINT      NOT NULL DEFAULT 0 CHECK (harga_charter >= 0),
    foto_url      TEXT        NOT NULL DEFAULT '',
    aktif         BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_tour_packages_merchant ON tour_packages (merchant_id);

CREATE TABLE IF NOT EXISTS charter_buses (
    id             UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    merchant_id    UUID        REFERENCES users(id) ON DELETE CASCADE,
    nama           TEXT        NOT NULL,
    tipe_bus       TEXT        NOT NULL DEFAULT '',
    kelas          TEXT        NOT NULL DEFAULT '',
    kapasitas      INTEGER     NOT NULL CHECK (kapasitas BETWEEN 1 AND 100),
    konfigurasi    TEXT        NOT NULL DEFAULT '',
    deskripsi      TEXT        NOT NULL DEFAULT '',
    fasilitas      TEXT[]      NOT NULL DEFAULT '{}',
    harga_harian   BIGINT      NOT NULL CHECK (harga_harian >= 0),
    catatan_harga  TEXT        NOT NULL DEFAULT '',
    foto_url       TEXT        NOT NULL DEFAULT '',
    aktif          BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_charter_buses_merchant ON charter_buses (merchant_id);

-- item_nama & merchant_id disalin saat permintaan dibuat: permintaan tetap
-- terbaca meski paket/bus-nya kemudian dihapus.
CREATE TABLE IF NOT EXISTS rental_requests (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    jenis            TEXT        NOT NULL CHECK (jenis IN ('paket', 'charter', 'custom')),
    item_id          UUID,
    item_nama        TEXT        NOT NULL DEFAULT '',
    merchant_id      UUID        REFERENCES users(id) ON DELETE SET NULL,
    user_id          UUID        REFERENCES users(id) ON DELETE SET NULL,
    nama             TEXT        NOT NULL,
    telp             TEXT        NOT NULL,
    jemput           TEXT        NOT NULL DEFAULT '',
    tujuan           TEXT        NOT NULL DEFAULT '',
    tgl_berangkat    DATE,
    tgl_pulang       DATE,
    tipe_perjalanan  TEXT        NOT NULL DEFAULT '',
    jumlah_orang     INTEGER     NOT NULL CHECK (jumlah_orang BETWEEN 1 AND 1000),
    catatan          TEXT        NOT NULL DEFAULT '',
    status           TEXT        NOT NULL DEFAULT 'baru'
                     CHECK (status IN ('baru', 'dihubungi', 'deal', 'batal')),
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_rental_requests_merchant ON rental_requests (merchant_id, created_at DESC);
