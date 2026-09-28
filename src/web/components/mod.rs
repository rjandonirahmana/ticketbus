pub mod app_chrome;
pub mod armada_manager;
pub mod auth_layout;
pub mod banner;
pub mod banner_manager;
pub mod calendar_grid;
pub mod fare_calendar;
pub mod icon;
pub mod map;
pub mod mitra;
pub mod mitra_review;
pub mod photo_gallery;
pub mod photo_upload;
pub mod pickup_picker;
pub mod radar;
pub mod rating_stars;
pub mod rental;
pub mod rental_manager;
pub mod route_manager;
pub mod schedule_card;
pub mod schedule_form;
pub mod schedule_modal;
pub mod theme_toggle;

pub use app_chrome::{AccountTabs, AppBar, BottomNav, BrandLogo, PageHead};
pub use armada_manager::ArmadaManager;
pub use auth_layout::{AuthLayout, AuthTab};
pub use banner::{BannerCarousel, BannerSlide};
pub use banner_manager::BannerManager;
pub use calendar_grid::CalendarGrid;
pub use fare_calendar::{harga_singkat, min_fare, week_start, FareCalendar};
pub use icon::Icon;
pub use map::{geo_clear, geo_watch, keep_awake, map_focus, map_init, map_on_select, map_pick, map_fit_all, map_set_items, map_set_user, map_set_user_label, map_zoom, now_ms, MapItem};
pub use mitra::{status_label as mitra_status_label, PoCard, ProfileFields, ProfileSignals};
pub use mitra_review::MitraReview;
pub use photo_gallery::PhotoGallery;
pub use photo_upload::PhotoUpload;
pub use pickup_picker::PickupPicker;
pub use radar::{build_rows, map_items, menit_ke_berangkat, use_nearby, use_user_location, NearbyRes, RadarCard, RadarRow};
pub use rating_stars::{RatingInput, RatingStars};
pub use rental::{wa_link, ImageUploadField, RentalRequestSheet, RequestPrefill, RequestTarget};
pub use rental_manager::{RentalManager, TextField};
pub use route_manager::RouteManager;
pub use schedule_card::{OpLogo, RouteTimeline, ScheduleCard, TripCard};
pub use schedule_form::ScheduleForm;
pub use schedule_modal::ScheduleModal;
pub use theme_toggle::ThemeToggle;

/// "1234567" → "1.234.567". Dipakai di mana pun harga (Rupiah) ditampilkan.
pub fn format_rupiah(n: i64) -> String {
    let neg = n < 0;
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    let out: String = out.chars().rev().collect();
    if neg {
        format!("-{out}")
    } else {
        out
    }
}

/// "Sinar Jaya Mandiri" → "SJ". Isi logo inisial operator & avatar.
pub fn initials(name: &str) -> String {
    let s: String = name
        .split_whitespace()
        .filter_map(|w| w.chars().find(|c| c.is_alphanumeric()))
        .take(2)
        .collect();
    if s.is_empty() {
        "?".into()
    } else {
        s.to_uppercase()
    }
}

const HARI: [&str; 7] = ["Sen", "Sel", "Rab", "Kam", "Jum", "Sab", "Min"];
const BULAN_PENDEK: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "Mei", "Jun", "Jul", "Agu", "Sep", "Okt", "Nov", "Des"];

/// "2026-09-25" → ("Jum", "25 Sep"). `None` kalau format tanggal tak dikenal.
pub fn tanggal_parts(iso: &str) -> Option<(&'static str, String)> {
    use chrono::Datelike;
    let d = chrono::NaiveDate::parse_from_str(iso, "%Y-%m-%d").ok()?;
    let hari = HARI[d.weekday().num_days_from_monday() as usize];
    Some((hari, format!("{} {}", d.day(), BULAN_PENDEK[d.month0() as usize])))
}

const HARI_PANJANG: [&str; 7] = ["Senin", "Selasa", "Rabu", "Kamis", "Jumat", "Sabtu", "Minggu"];

/// "2026-09-25" → "Jumat, 25 Sep 2026" (string asli bila gagal di-parse).
pub fn format_tanggal_panjang(iso: &str) -> String {
    use chrono::Datelike;
    match chrono::NaiveDate::parse_from_str(iso, "%Y-%m-%d") {
        Ok(d) => format!(
            "{}, {} {} {}",
            HARI_PANJANG[d.weekday().num_days_from_monday() as usize],
            d.day(),
            BULAN_PENDEK[d.month0() as usize],
            d.year()
        ),
        Err(_) => iso.to_string(),
    }
}

/// "2026-09-25" → "Jum, 25 Sep 2026" (string asli bila gagal di-parse).
pub fn format_tanggal(iso: &str) -> String {
    match tanggal_parts(iso) {
        Some((hari, tgl)) => format!("{hari}, {tgl} {}", &iso[..4]),
        None => iso.to_string(),
    }
}

/// Tanggal "hari ini" menurut WIB (UTC+7) — server & klien menghitung sama
/// supaya kalender beranda tidak beda saat hidrasi.
pub fn today_wib() -> chrono::NaiveDate {
    (chrono::Utc::now() + chrono::Duration::hours(7)).date_naive()
}

/// Geser (tahun, bulan) sebanyak `delta` bulan.
pub fn shift_month(year: i32, month: u32, delta: i32) -> (i32, u32) {
    let total = year * 12 + month as i32 - 1 + delta;
    let y = total.div_euclid(12);
    let m = total.rem_euclid(12) + 1;
    (y, m as u32)
}

const BULAN: [&str; 12] = [
    "Januari", "Februari", "Maret", "April", "Mei", "Juni", "Juli", "Agustus", "September", "Oktober", "November",
    "Desember",
];

/// (2026, 9) → "September 2026".
pub fn month_label(year: i32, month: u32) -> String {
    let idx = (month as usize).saturating_sub(1).min(11);
    format!("{} {year}", BULAN[idx])
}

/// Buang bungkus teknis dari pesan server fn supaya yang sampai ke pengguna
/// adalah kalimat yang ditulis service, bukan jejak internal Leptos.
pub fn clean_error(raw: &str) -> String {
    // Balasan error TANPA isi (mis. 502 dari proxy saat aplikasi mati/restart)
    // tak bisa dibaca sebagai error server fn.
    if raw.contains("missing delimiter in \"\"") {
        return "Server tidak merespons (aplikasi mungkin sedang restart). Coba lagi sebentar lagi.".into();
    }
    raw.trim_start_matches("error running server function: ")
        .trim_start_matches("ServerFnError: ")
        .trim()
        .to_string()
}

/// Jalankan tugas async di klien saja. Di server ini no-op — `spawn_local`
/// tidak tersedia (dan tidak ada gunanya) di luar wasm.
pub(crate) fn spawn_client<F>(fut: F)
where
    F: std::future::Future<Output = ()> + 'static,
{
    #[cfg(target_arch = "wasm32")]
    leptos::task::spawn_local(fut);
    #[cfg(not(target_arch = "wasm32"))]
    drop(fut);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rupiah_terformat() {
        assert_eq!(format_rupiah(1234567), "1.234.567");
        assert_eq!(format_rupiah(0), "0");
        assert_eq!(format_rupiah(999), "999");
        assert_eq!(format_rupiah(-5000), "-5.000");
    }

    #[test]
    fn inisial_dan_tanggal() {
        assert_eq!(initials("Sinar Jaya Mandiri"), "SJ");
        assert_eq!(initials("  lorena "), "L");
        assert_eq!(initials(""), "?");
        assert_eq!(format_tanggal("2026-09-25"), "Jum, 25 Sep 2026");
        assert_eq!(format_tanggal("besok"), "besok");
        assert_eq!(format_tanggal_panjang("2026-09-24"), "Kamis, 24 Sep 2026");
        assert_eq!(shift_month(2026, 1, -1), (2025, 12));
        assert_eq!(shift_month(2026, 12, 1), (2027, 1));
        assert_eq!(month_label(2026, 9), "September 2026");
        assert!(clean_error("error deserializing server function results: Invalid format: missing delimiter in \"\"")
            .starts_with("Server tidak merespons"));
    }
}
