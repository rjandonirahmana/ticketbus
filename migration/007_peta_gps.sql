-- 007_peta_gps.sql — "Radar Bus di Sekitar" (beranda) & halaman Peta Bus.
--
-- Titik jemput kini punya koordinat (dipilih mitra di peta; opsional).
-- tracking_token = rahasia tautan driver (/driver/:token): siapa pun yang
-- memegangnya bisa mengirim posisi bus untuk jadwal itu, jadi token TIDAK
-- pernah ikut DTO publik — hanya diberikan ke pemilik jadwal & admin.
ALTER TABLE schedules
    ADD COLUMN IF NOT EXISTS jemput_lat      DOUBLE PRECISION,
    ADD COLUMN IF NOT EXISTS jemput_lng      DOUBLE PRECISION,
    ADD COLUMN IF NOT EXISTS tracking_token  TEXT;

UPDATE schedules
   SET tracking_token = replace(gen_random_uuid()::text, '-', '')
 WHERE tracking_token IS NULL;

ALTER TABLE schedules
    ALTER COLUMN tracking_token SET DEFAULT replace(gen_random_uuid()::text, '-', ''),
    ALTER COLUMN tracking_token SET NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_schedules_tracking_token ON schedules (tracking_token);

-- Posisi TERAKHIR bus per jadwal (bukan riwayat) — di-upsert dari HP driver
-- tiap ±10 detik. Posisi yang lebih tua dari 10 menit dianggap sinyal hilang.
CREATE TABLE IF NOT EXISTS bus_positions (
    schedule_id  UUID             PRIMARY KEY REFERENCES schedules(id) ON DELETE CASCADE,
    lat          DOUBLE PRECISION NOT NULL CHECK (lat BETWEEN -90 AND 90),
    lng          DOUBLE PRECISION NOT NULL CHECK (lng BETWEEN -180 AND 180),
    speed_kmh    DOUBLE PRECISION,
    heading      DOUBLE PRECISION,
    accuracy_m   DOUBLE PRECISION,
    updated_at   TIMESTAMPTZ      NOT NULL DEFAULT NOW()
);
