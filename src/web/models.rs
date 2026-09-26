//! web/models.rs — DTO yang dipakai server function DAN UI (kompil di kedua
//! target, native + wasm). Tanggal/waktu disimpan sebagai `String` (bukan
//! `chrono::NaiveDate`/`Uuid`) supaya tipe ini tetap sesederhana mungkin di
//! kedua sisi — repository (native-only) yang mengonversi dari/ke tipe
//! Postgres asli.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Armada {
    pub id: String,
    pub name: String,
    pub color_hex: String,
    pub urutan: i32,
    /// `None` = milik platform/admin (bukan armada satu merchant tertentu).
    pub merchant_id: Option<String>,
    pub avg_rating_armada: f64,
    pub avg_rating_driver: f64,
    pub jumlah_rating: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Schedule {
    pub id: String,
    pub armada_id: String,
    /// Denormalized supaya halaman browse/order tak perlu join manual di UI.
    pub armada_name: String,
    pub armada_color_hex: String,
    /// Format `YYYY-MM-DD`.
    pub tanggal: String,
    pub tujuan: String,
    pub lokasi_jemput: String,
    pub jam: String,
    /// Rupiah, satuan penuh (bukan sen).
    pub harga: i64,
    pub catatan: String,
    pub kapasitas: i32,
    pub kursi_terjual: i32,
    pub driver_nama: String,
    pub driver_telp: String,
    /// Tata letak kursi: `2-2` | `2-1` | `1-1` (lihat `web::seats`).
    pub konfigurasi: String,
    pub dua_dek: bool,
    /// Kode kursi yang diprioritaskan untuk penumpang wanita.
    pub kursi_wanita: Vec<String>,
}

impl Schedule {
    pub fn sisa_kursi(&self) -> i32 {
        (self.kapasitas - self.kursi_terjual).max(0)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TripPhoto {
    pub id: String,
    pub armada_id: String,
    pub armada_name: String,
    pub url: String,
    pub caption: String,
    /// Format RFC3339.
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct NewSchedule {
    pub armada_id: String,
    pub tanggal: String,
    pub tujuan: String,
    pub lokasi_jemput: String,
    pub jam: String,
    pub harga: i64,
    pub catatan: String,
    pub kapasitas: i32,
    pub driver_nama: String,
    pub driver_telp: String,
    pub konfigurasi: String,
    pub dua_dek: bool,
    /// Kode kursi wanita dipisah koma (form-urlencoded server fn).
    pub kursi_wanita: String,
}

// ── Akun & sesi ────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PublicUser {
    pub id: String,
    pub name: String,
    pub phone: String,
    /// "buyer" | "merchant" | "admin".
    pub role: String,
}

// ── Order & tiket ────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct NewOrder {
    pub schedule_id: String,
    /// Diabaikan server bila `kursi` diisi — jumlah = banyaknya kursi.
    pub jumlah_tiket: i32,
    /// Kode kursi dipisah koma, mis. "03A,03B".
    pub kursi: String,
    pub nama_pemesan: String,
    pub telp_pemesan: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OrderSummary {
    pub id: String,
    pub kode_order: String,
    pub tanggal: String,
    pub tujuan: String,
    pub armada_name: String,
    pub jumlah_tiket: i32,
    pub total_harga: i64,
    pub status: String,
    pub created_at: String,
    pub sudah_dirating: bool,
    /// Nomor kursi (kosong untuk order sebelum kursi bernomor ada).
    pub kursi: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OrderDetail {
    pub id: String,
    pub kode_order: String,
    pub armada_id: String,
    pub armada_name: String,
    pub tanggal: String,
    pub tujuan: String,
    pub lokasi_jemput: String,
    pub jam: String,
    pub driver_nama: String,
    pub driver_telp: String,
    pub jumlah_tiket: i32,
    pub harga_satuan: i64,
    pub total_harga: i64,
    pub nama_pemesan: String,
    pub telp_pemesan: String,
    pub status: String,
    pub created_at: String,
    pub sudah_dirating: bool,
    pub kursi: Vec<String>,
}

/// Denah untuk halaman Pilih Kursi: jadwal + kode kursi yang sudah terjual.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SeatMap {
    pub schedule: Schedule,
    pub terisi: Vec<String>,
}

// ── Keamanan akun ────────────────────────────────────────────────────────────

/// Satu sesi login (perangkat) milik pengguna.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SessionInfo {
    pub id: String,
    pub perangkat: String,
    pub mobile: bool,
    pub ip: String,
    pub created_at: String,
    pub last_seen_at: String,
    /// Sesi yang sedang dipakai membuka halaman ini.
    pub saat_ini: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SecurityEvent {
    pub jenis: String,
    pub perangkat: String,
    pub ip: String,
    pub created_at: String,
}

/// Faktor skor keamanan — dihitung dari data asli, bukan angka hiasan.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SecurityFactor {
    pub label: String,
    pub poin: i32,
    pub maks: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SecurityOverview {
    pub skor: i32,
    pub faktor: Vec<SecurityFactor>,
    pub phone: String,
    pub password_changed_at: String,
    pub login_gagal_30h: i64,
    pub sesi: Vec<SessionInfo>,
    pub riwayat: Vec<SecurityEvent>,
}

// ── Rating ───────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct NewRating {
    pub order_id: String,
    pub rating_armada: i32,
    pub rating_driver: i32,
    pub komentar: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Rating {
    pub id: String,
    pub order_id: String,
    pub rating_armada: i32,
    pub rating_driver: i32,
    pub komentar: String,
    pub created_at: String,
}

// ── Sewa bus & paket wisata ──────────────────────────────────────────────────

/// Paket bus destinasi wisata (halaman `/wisata`). `merchant_id = None` =
/// paket milik platform (dibuat admin).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TourPackage {
    pub id: String,
    pub merchant_id: Option<String>,
    /// Nama pemilik (nama mitra PO, atau "LajuBus" untuk paket platform).
    pub pemilik: String,
    pub judul: String,
    /// `alam` | `budaya` | `religi` | `edukasi` | `lainnya`.
    pub kategori: String,
    pub kawasan: String,
    pub durasi: String,
    pub label_badge: String,
    pub label_tipe: String,
    pub rute: String,
    pub armada_info: String,
    pub fasilitas: Vec<String>,
    pub harga_pax: i64,
    pub min_pax: i32,
    /// 0 = tidak ditawarkan sebagai charter bus saja.
    pub harga_charter: i64,
    pub foto_url: String,
    pub aktif: bool,
}

/// Input form paket. `fasilitas` = teks dipisah koma/baris baru — lebih aman
/// dikirim lewat form-urlencoded server fn daripada `Vec`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct NewTourPackage {
    pub judul: String,
    pub kategori: String,
    pub kawasan: String,
    pub durasi: String,
    pub label_badge: String,
    pub label_tipe: String,
    pub rute: String,
    pub armada_info: String,
    pub fasilitas: String,
    pub harga_pax: i64,
    pub min_pax: i32,
    pub harga_charter: i64,
    pub foto_url: String,
}

/// Bus di katalog charter (halaman `/sewa`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CharterBus {
    pub id: String,
    pub merchant_id: Option<String>,
    pub pemilik: String,
    pub nama: String,
    pub tipe_bus: String,
    pub kelas: String,
    pub kapasitas: i32,
    pub konfigurasi: String,
    pub deskripsi: String,
    pub fasilitas: Vec<String>,
    pub harga_harian: i64,
    pub catatan_harga: String,
    pub foto_url: String,
    pub aktif: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct NewCharterBus {
    pub nama: String,
    pub tipe_bus: String,
    pub kelas: String,
    pub kapasitas: i32,
    pub konfigurasi: String,
    pub deskripsi: String,
    pub fasilitas: String,
    pub harga_harian: i64,
    pub catatan_harga: String,
    pub foto_url: String,
}

/// Permintaan sewa/booking dari penumpang. `jenis`: `paket` | `charter` |
/// `custom` (rencana wisata sendiri, tanpa item).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RentalRequest {
    pub id: String,
    pub jenis: String,
    pub item_nama: String,
    pub nama: String,
    pub telp: String,
    pub jemput: String,
    pub tujuan: String,
    pub tgl_berangkat: String,
    pub tgl_pulang: String,
    pub tipe_perjalanan: String,
    pub jumlah_orang: i32,
    pub catatan: String,
    /// `baru` | `dihubungi` | `deal` | `batal`.
    pub status: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct NewRentalRequest {
    pub jenis: String,
    /// Kosong untuk `custom`.
    pub item_id: String,
    pub nama: String,
    pub telp: String,
    pub jemput: String,
    pub tujuan: String,
    /// "YYYY-MM-DD" atau kosong.
    pub tgl_berangkat: String,
    pub tgl_pulang: String,
    pub tipe_perjalanan: String,
    pub jumlah_orang: i32,
    pub catatan: String,
}
