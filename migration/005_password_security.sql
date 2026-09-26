-- 005_password_security.sql — halaman "Ganti Kata Sandi" (/akun/password).
--
-- password_changed_at  : ditampilkan sebagai "Terakhir diubah". NULL = belum
--                        pernah diubah sejak akun dibuat (pakai created_at).
-- sessions_valid_after : "Keluarkan dari semua perangkat lain". Token sesi
--                        yang TERBIT sebelum waktu ini ditolak. Dimuat ke
--                        memori saat start (AuthService) supaya verifikasi
--                        cookie tetap tanpa query DB per request.
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS password_changed_at  TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS sessions_valid_after TIMESTAMPTZ;
