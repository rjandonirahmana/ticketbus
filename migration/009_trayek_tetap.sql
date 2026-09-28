-- 009_trayek_tetap.sql — trayek (rute) tetap yang berulang tiap hari.
--
-- Satu trayek = satu arah perjalanan dengan jam tetap, mis. Solo → Jakarta
-- berangkat 17:00 tiba 05:00 (esok hari). Rute balik adalah trayek kedua
-- yang ditautkan lewat `pasangan_id`. Dari trayek, server membuat baris
-- `schedules` untuk N hari ke depan (lihat service/route.rs) — order, kursi,
-- dan GPS tetap memakai `schedules` apa adanya. Yang berubah per hari hanya
-- bus (armada) & driver, diubah langsung di baris `schedules` hari itu.
--
-- Pemilik: merchant_id (NULL = milik platform/admin), bukan diturunkan dari
-- armada, supaya bus default boleh dihapus tanpa ikut menghapus trayeknya.
CREATE TABLE IF NOT EXISTS routes (
    id             UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    merchant_id    UUID        REFERENCES users(id) ON DELETE CASCADE,
    pasangan_id    UUID        REFERENCES routes(id) ON DELETE SET NULL,
    asal           TEXT        NOT NULL,
    tujuan         TEXT        NOT NULL,
    lokasi_jemput  TEXT        NOT NULL DEFAULT '',
    jemput_lat     DOUBLE PRECISION,
    jemput_lng     DOUBLE PRECISION,
    -- "HH:MM" WIB. Jam tiba <= jam berangkat berarti tiba esok hari.
    jam_berangkat  TEXT        NOT NULL,
    jam_tiba       TEXT        NOT NULL DEFAULT '',
    harga          BIGINT      NOT NULL CHECK (harga >= 0),
    kapasitas      INTEGER     NOT NULL CHECK (kapasitas BETWEEN 1 AND 200),
    konfigurasi    TEXT        NOT NULL DEFAULT '2-2' CHECK (konfigurasi IN ('2-2', '2-1', '1-1')),
    dua_dek        BOOLEAN     NOT NULL DEFAULT FALSE,
    kursi_wanita   TEXT[]      NOT NULL DEFAULT '{}',
    catatan        TEXT        NOT NULL DEFAULT '',
    -- Bus & driver default untuk hari-hari baru; bisa diganti per hari.
    armada_id      UUID        REFERENCES armadas(id) ON DELETE SET NULL,
    driver_nama    TEXT        NOT NULL DEFAULT '',
    driver_telp    TEXT        NOT NULL DEFAULT '',
    -- Bit 0 = Senin … bit 6 = Minggu. 127 = setiap hari.
    hari_operasi   SMALLINT    NOT NULL DEFAULT 127 CHECK (hari_operasi BETWEEN 1 AND 127),
    aktif          BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_routes_merchant ON routes (merchant_id);

ALTER TABLE schedules
    ADD COLUMN IF NOT EXISTS route_id  UUID    REFERENCES routes(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS asal      TEXT    NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS jam_tiba  TEXT    NOT NULL DEFAULT '',
    -- Keberangkatan hari itu dibatalkan: disembunyikan dari penumpang tapi
    -- barisnya tetap ada supaya generator tidak membuatnya lagi.
    ADD COLUMN IF NOT EXISTS batal     BOOLEAN NOT NULL DEFAULT FALSE;

-- Satu keberangkatan per trayek per tanggal (kunci ON CONFLICT generator).
CREATE UNIQUE INDEX IF NOT EXISTS idx_schedules_route_tanggal ON schedules (route_id, tanggal);
