//! service/wa_message.rs — template pesan WhatsApp LajuBus. Dipisah dari
//! `auth.rs` supaya wording mudah diubah & diuji tanpa menyentuh alurnya.
//!
//! Format WhatsApp: `*tebal*`, `_miring_`, ```` ```monospace``` ```` —
//! kode OTP & password dibungkus monospace agar mudah disalin dan tidak
//! tertukar antara huruf O dan angka 0.

use crate::web::models::RentalRequest;

const BRAND: &str = "🚌 *LajuBus* — Tiket & Jadwal Bis";
const FOOTER: &str = "_Pesan otomatis LajuBus. Jangan balas pesan ini._";
const JANGAN_BAGIKAN: &str = "🔒 *Jangan bagikan* kode/password ini kepada siapa pun, termasuk yang mengaku petugas LajuBus.";

fn peran_label(role: &str) -> &'static str {
    match role {
        "merchant" => "Mitra PO",
        "admin" => "Admin",
        _ => "Penumpang",
    }
}

/// Pendaftaran akun baru: OTP + password awal.
pub fn register(name: &str, role: &str, otp: &str, ttl_menit: i64, password: &str) -> String {
    format!(
        "{BRAND}\n\n\
         Halo *{name}* 👋\n\
         Terima kasih telah mendaftar sebagai *{peran}* di LajuBus.\n\n\
         🔐 *Kode OTP pendaftaran:*\n\
         ```{otp}```\n\
         ⏳ Berlaku *{ttl_menit} menit*.\n\n\
         🔑 *Password akun Anda:*\n\
         ```{password}```\n\
         Simpan baik-baik — dipakai untuk masuk berikutnya bersama nomor WhatsApp ini.\n\n\
         {JANGAN_BAGIKAN}\n\n\
         {FOOTER}",
        peran = peran_label(role),
    )
}

/// Kirim ulang OTP pendaftaran (password awal tidak berubah).
pub fn resend_otp(name: &str, otp: &str, ttl_menit: i64) -> String {
    format!(
        "{BRAND}\n\n\
         Halo *{name}* 👋\n\
         Berikut kode OTP *baru* untuk menyelesaikan pendaftaran Anda.\n\n\
         🔐 *Kode OTP:*\n\
         ```{otp}```\n\
         ⏳ Berlaku *{ttl_menit} menit*. Kode sebelumnya sudah tidak berlaku.\n\n\
         ℹ️ Password akun tetap sama seperti pada pesan pendaftaran sebelumnya.\n\n\
         {JANGAN_BAGIKAN}\n\n\
         {FOOTER}"
    )
}

/// Lupa password: password baru tertunda, password lama tetap berlaku.
pub fn forgot_password(name: &str, password: &str, ttl_jam: i64) -> String {
    format!(
        "{BRAND}\n\n\
         Halo *{name}* 👋\n\
         Kami menerima permintaan *reset password* untuk akun LajuBus Anda.\n\n\
         🔑 *Password baru:*\n\
         ```{password}```\n\
         ⏳ Aktifkan dengan login dalam *{ttl_jam} jam*.\n\n\
         ✅ Password lama *tetap berlaku* sampai password baru dipakai login.\n\
         ⚠️ Tidak merasa meminta? Abaikan pesan ini — akun Anda tetap aman.\n\n\
         {JANGAN_BAGIKAN}\n\n\
         {FOOTER}"
    )
}

/// Ganti nomor HP: OTP dikirim ke nomor BARU.
pub fn phone_change(otp: &str, ttl_menit: i64) -> String {
    format!(
        "{BRAND}\n\n\
         📱 *Verifikasi Nomor WhatsApp Baru*\n\
         Nomor ini akan dijadikan nomor login akun LajuBus Anda.\n\n\
         🔐 *Kode OTP:*\n\
         ```{otp}```\n\
         ⏳ Berlaku *{ttl_menit} menit*. Masukkan di halaman *Akun Saya*.\n\n\
         ⚠️ Tidak merasa meminta? Abaikan pesan ini — nomor tidak akan diganti.\n\n\
         {JANGAN_BAGIKAN}\n\n\
         {FOOTER}"
    )
}

/// Pemberitahuan keamanan: password baru saja diganti dari halaman Akun.
pub fn password_changed(name: &str, logout_others: bool) -> String {
    let sesi = if logout_others {
        "📱 Semua perangkat lain sudah *dikeluarkan* dari akun Anda.\n"
    } else {
        ""
    };
    format!(
        "{BRAND}\n\n\
         Halo *{name}* 👋\n\
         🔐 Kata sandi akun LajuBus Anda *baru saja diubah*.\n\
         {sesi}\n\
         ✅ Bila ini Anda, tak perlu melakukan apa pun.\n\
         ⚠️ Bukan Anda? Segera gunakan *Lupa Password* di halaman masuk dan hubungi CS LajuBus.\n\n\
         {FOOTER}"
    )
}

/// Pemberitahuan: akun dibekukan dari Pusat Keamanan.
pub fn account_frozen(name: &str) -> String {
    format!(
        "{BRAND}\n\n\
         Halo *{name}* 👋\n\
         🧊 Akun LajuBus Anda *dibekukan sementara* dan semua perangkat sudah dikeluarkan.\n\n\
         🔓 Untuk membukanya kembali: buka halaman masuk → *Lupa password?* → masukkan nomor ini. \
         Password baru akan dikirim lewat WhatsApp, dan login dengannya membuka akun Anda.\n\n\
         ⚠️ Bukan Anda yang membekukan? Segera hubungi CS LajuBus.\n\n\
         {FOOTER}"
    )
}

fn jenis_label(jenis: &str) -> &'static str {
    match jenis {
        "paket" => "Paket Wisata",
        "charter" => "Sewa Bus Charter",
        _ => "Rencana Wisata Custom",
    }
}

/// Baris-baris ringkasan permintaan sewa (dipakai pesan pelanggan & CS).
fn rental_summary(r: &RentalRequest) -> String {
    let mut lines = vec![format!("📋 *{}*: {}", jenis_label(&r.jenis), r.item_nama)];
    if !r.jemput.is_empty() {
        lines.push(format!("📍 Jemput: {}", r.jemput));
    }
    if !r.tujuan.is_empty() {
        lines.push(format!("🏁 Tujuan: {}", r.tujuan));
    }
    if !r.tgl_berangkat.is_empty() {
        let tanggal = crate::web::components::format_tanggal(&r.tgl_berangkat);
        match r.tgl_pulang.as_str() {
            "" => lines.push(format!("📅 Berangkat: {tanggal}")),
            pulang => lines.push(format!(
                "📅 {tanggal} → {}",
                crate::web::components::format_tanggal(pulang)
            )),
        }
    }
    if !r.tipe_perjalanan.is_empty() {
        lines.push(format!("🔁 Perjalanan: {}", r.tipe_perjalanan));
    }
    lines.push(format!("👥 Rombongan: *{} orang*", r.jumlah_orang));
    if !r.catatan.is_empty() {
        lines.push(format!("📝 Catatan: _{}_", r.catatan));
    }
    lines.join("\n")
}

/// Konfirmasi ke pelanggan bahwa permintaan sewa sudah diterima.
pub fn rental_received(r: &RentalRequest) -> String {
    format!(
        "{BRAND}\n\n\
         Halo *{nama}* 👋\n\
         Terima kasih! Permintaan sewa bus Anda sudah kami terima ✅\n\n\
         {ringkasan}\n\n\
         🕑 Tim LajuBus / mitra PO akan menghubungi Anda lewat WhatsApp ini untuk penawaran harga & ketersediaan armada.\n\
         💬 Ada tambahan info? Hubungi CS lewat tombol *Konsultasi WhatsApp* di aplikasi.\n\n\
         {FOOTER}",
        nama = r.nama,
        ringkasan = rental_summary(r),
    )
}

/// Notifikasi ke CS/admin: ada permintaan sewa baru.
pub fn rental_to_cs(r: &RentalRequest) -> String {
    format!(
        "🔔 *Permintaan Sewa Baru — LajuBus*\n\n\
         👤 {nama} · wa.me/{telp}\n\
         {ringkasan}\n\n\
         Tindak lanjuti di dashboard: menu *Sewa & Wisata → Permintaan*.",
        nama = r.nama,
        telp = r.telp,
        ringkasan = rental_summary(r),
    )
}

/// Ke CS/admin: ada pendaftaran Mitra PO baru yang perlu direview.
pub fn mitra_baru_ke_cs(nama_po: &str, pic: &str, telp: &str, kota: &str) -> String {
    format!(
        "🔔 *Pendaftaran Mitra PO Baru — LajuBus*\n\n\
         🚌 *{nama_po}*{kota}\n\
         👤 {pic} · wa.me/{telp}\n\n\
         Review & setujui di Dashboard Admin → tab *Mitra PO*.\n\n\
         {FOOTER}",
        kota = if kota.is_empty() { String::new() } else { format!(" — {kota}") },
    )
}

/// Ke mitra: hasil review pendaftaran.
pub fn mitra_diputuskan(nama_po: &str, disetujui: bool, catatan: &str) -> String {
    if disetujui {
        format!(
            "{BRAND}\n\n\
             Selamat! 🎉 *{nama_po}* sudah *terverifikasi* sebagai Mitra PO LajuBus.\n\n\
             Trayek & jadwal Anda kini tampil di beranda dan bisa dipesan penumpang.\n\n\
             {FOOTER}"
        )
    } else {
        format!(
            "{BRAND}\n\n\
             Pendaftaran *{nama_po}* sebagai Mitra PO *belum dapat disetujui*.\n\n\
             📝 Catatan admin: {catatan}\n\n\
             Perbaiki profil PO di menu *Mitra → Profil PO*, lalu simpan untuk mengajukan ulang.\n\n\
             {FOOTER}",
            catatan = if catatan.trim().is_empty() { "-" } else { catatan.trim() },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pesan_memuat_data_penting() {
        let r = register("Budi", "merchant", "123456", 10, "Ab3xY9");
        assert!(r.contains("*Budi*") && r.contains("```123456```") && r.contains("```Ab3xY9```"));
        assert!(r.contains("*Mitra PO*") && r.contains("10 menit") && r.contains("LajuBus"));

        let o = resend_otp("Sari", "654321", 10);
        assert!(o.contains("```654321```") && o.contains("*Sari*"));

        let f = forgot_password("Sari", "Zz9Kq1", 24);
        assert!(f.contains("```Zz9Kq1```") && f.contains("24 jam") && f.contains("tetap berlaku"));

        let req = crate::web::models::RentalRequest {
            id: "x".into(),
            jenis: "charter".into(),
            item_nama: "Bus A (50 seat)".into(),
            nama: "Rina".into(),
            telp: "6281234567890".into(),
            jemput: "Jakarta".into(),
            tujuan: "Bandung".into(),
            tgl_berangkat: "2026-10-18".into(),
            tgl_pulang: "".into(),
            tipe_perjalanan: "Pulang-Pergi".into(),
            jumlah_orang: 45,
            catatan: "".into(),
            status: "baru".into(),
            created_at: "".into(),
        };
        let c = rental_received(&req);
        assert!(c.contains("*Rina*") && c.contains("45 orang") && c.contains("Bus A"));
        assert!(rental_to_cs(&req).contains("wa.me/6281234567890"));

        let p = phone_change("111222", 10);
        assert!(p.contains("```111222```") && p.contains("Akun Saya"));
    }
}
