-- 010_mitra_po.sql — pendaftaran Mitra PO dengan persetujuan admin + profil
-- PO publik (/po/:id).
--
-- Satu profil per akun merchant. Status:
--   menunggu  → baru daftar / mengajukan ulang; jadwal, trayek, dan paket
--               milik PO ini TIDAK tampil ke penumpang & tak bisa dipesan;
--   disetujui → tampil normal (badge "Terverifikasi");
--   ditolak   → catatan_admin berisi alasan; mitra bisa memperbaiki profil
--               lalu mengajukan ulang (kembali ke `menunggu`).
CREATE TABLE IF NOT EXISTS merchant_profiles (
    user_id        UUID        PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    nama_po        TEXT        NOT NULL,
    kota           TEXT        NOT NULL DEFAULT '',
    alamat         TEXT        NOT NULL DEFAULT '',
    deskripsi      TEXT        NOT NULL DEFAULT '',
    tahun_berdiri  INTEGER     CHECK (tahun_berdiri BETWEEN 1900 AND 2100),
    -- Jumlah armada yang DIKLAIM saat mendaftar (armada sungguhan dihitung
    -- dari tabel armadas).
    jumlah_armada  INTEGER     NOT NULL DEFAULT 0 CHECK (jumlah_armada BETWEEN 0 AND 10000),
    layanan        TEXT[]      NOT NULL DEFAULT '{}',
    -- Data review (tidak tampil di profil publik).
    nama_pemilik   TEXT        NOT NULL DEFAULT '',
    email          TEXT        NOT NULL DEFAULT '',
    nomor_izin     TEXT        NOT NULL DEFAULT '',
    dokumen_url    TEXT        NOT NULL DEFAULT '',
    logo_url       TEXT        NOT NULL DEFAULT '',
    sampul_url     TEXT        NOT NULL DEFAULT '',
    status         TEXT        NOT NULL DEFAULT 'menunggu'
                   CHECK (status IN ('menunggu', 'disetujui', 'ditolak')),
    catatan_admin  TEXT        NOT NULL DEFAULT '',
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    diputuskan_at  TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_merchant_profiles_status ON merchant_profiles (status, created_at DESC);

-- Mitra yang sudah ada sebelum fitur ini dianggap sudah terverifikasi.
INSERT INTO merchant_profiles (user_id, nama_po, status, diputuskan_at)
SELECT id, name, 'disetujui', NOW() FROM users WHERE role = 'merchant'
ON CONFLICT (user_id) DO NOTHING;

-- Draf profil PO (JSON NewMerchantProfile) selama menunggu OTP pendaftaran.
ALTER TABLE otp_pending ADD COLUMN IF NOT EXISTS profil_po TEXT;
