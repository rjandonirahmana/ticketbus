-- 003_account_recovery.sql — lupa password & ganti nomor HP, keduanya via WA.

-- Password baru dari "lupa password" TIDAK langsung menggantikan yang lama:
-- disimpan di sini dan baru dipromosikan saat benar-benar dipakai login.
-- Tanpa ini, siapa pun yang mengetik nomor orang lain di form lupa password
-- bisa mengunci pemiliknya keluar dari akunnya sendiri.
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS pending_password_hash       TEXT,
    ADD COLUMN IF NOT EXISTS pending_password_expires_at TIMESTAMPTZ;

-- Ganti nomor: OTP dikirim ke nomor BARU, nomor baru baru dipakai setelah
-- OTP-nya terbukti diterima di sana.
CREATE TABLE IF NOT EXISTS phone_change_pending (
    user_id     UUID        PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    new_phone   TEXT        NOT NULL,
    otp_code    TEXT        NOT NULL,
    attempts    INTEGER     NOT NULL DEFAULT 0,
    expires_at  TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

