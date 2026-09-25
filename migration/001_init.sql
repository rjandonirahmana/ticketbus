-- 001_init.sql — skema awal fleet management PO Nyentrix Trans.

CREATE TABLE IF NOT EXISTS armadas (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT        NOT NULL,
    color_hex   TEXT        NOT NULL DEFAULT '#2563eb',
    urutan      INTEGER     NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS schedules (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    armada_id       UUID        NOT NULL REFERENCES armadas(id) ON DELETE CASCADE,
    tanggal         DATE        NOT NULL,
    tujuan          TEXT        NOT NULL,
    lokasi_jemput   TEXT        NOT NULL DEFAULT '',
    jam             TEXT        NOT NULL DEFAULT '',
    harga           BIGINT      NOT NULL DEFAULT 0,
    catatan         TEXT        NOT NULL DEFAULT '',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_schedules_tanggal ON schedules (tanggal);
CREATE INDEX IF NOT EXISTS idx_schedules_armada_id ON schedules (armada_id);

CREATE TABLE IF NOT EXISTS trip_photos (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    armada_id   UUID        NOT NULL REFERENCES armadas(id) ON DELETE CASCADE,
    url         TEXT        NOT NULL,
    caption     TEXT        NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_trip_photos_armada_id ON trip_photos (armada_id);

-- Seed 4 armada awal, meniru "Armada 1..4" situs asli — sudah bisa
-- diubah/dihapus/ditambah bebas lewat panel admin sejak baris ini.
INSERT INTO armadas (name, color_hex, urutan)
SELECT * FROM (VALUES
    ('Armada 1', '#2563eb', 1),
    ('Armada 2', '#dc2626', 2),
    ('Armada 3', '#9333ea', 3),
    ('Armada 4', '#111827', 4)
) AS seed(name, color_hex, urutan)
WHERE NOT EXISTS (SELECT 1 FROM armadas);
