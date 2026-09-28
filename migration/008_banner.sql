-- 008_banner.sql — banner "Promo & Info Spesial" di beranda, dikelola admin
-- (tab Banner di /admin). Banner boleh berupa gambar saja (teks sudah ada di
-- gambar → judul kosong), teks di atas gambar, atau teks di atas gradasi
-- `tema` bila tanpa gambar. Jadwal tayang opsional: mulai/selesai NULL =
-- tanpa batas; dibandingkan dengan tanggal WIB.
CREATE TABLE IF NOT EXISTS banners (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    judul       TEXT        NOT NULL DEFAULT '',
    subjudul    TEXT        NOT NULL DEFAULT '',
    label       TEXT        NOT NULL DEFAULT '',
    kode_promo  TEXT        NOT NULL DEFAULT '',
    cta_label   TEXT        NOT NULL DEFAULT '',
    link_url    TEXT        NOT NULL DEFAULT '',
    gambar_url  TEXT        NOT NULL DEFAULT '',
    tema        TEXT        NOT NULL DEFAULT 'sapphire'
                CHECK (tema IN ('sapphire', 'malam', 'emerald', 'tangerine')),
    mulai       DATE,
    selesai     DATE,
    urutan      INTEGER     NOT NULL DEFAULT 0,
    aktif       BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (judul <> '' OR gambar_url <> ''),
    CHECK (mulai IS NULL OR selesai IS NULL OR selesai >= mulai)
);
CREATE INDEX IF NOT EXISTS idx_banners_urutan ON banners (urutan, created_at DESC);
