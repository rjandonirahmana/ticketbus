//! web/seats.rs — denah kursi bus, dikompilasi untuk KEDUA target.
//!
//! Satu-satunya sumber kebenaran tata letak: UI menggambar denah dari sini,
//! dan server (repository order) memvalidasi kode kursi yang dipesan dengan
//! fungsi yang sama — kode yang tak ada di denah tak mungkin terjual.
//!
//! Penomoran: baris 01, 02, … (berlanjut lintas dek), huruf kiri→kanan per
//! baris. Konfigurasi `2-2` = A B | C D, `2-1` = A B | C, `1-1` = A | B.
//! Kursi paling kiri & paling kanan tiap baris = sisi jendela.

/// Satu kursi pada denah.
#[derive(Clone, Debug, PartialEq)]
pub struct Seat {
    pub kode: String,
    /// 1 = dek bawah, 2 = dek atas.
    pub dek: u8,
    pub baris: u32,
    /// Indeks kolom 0.. dari kiri (lorong ada di antara `kiri` dan kanan).
    pub kolom: u8,
    pub jendela: bool,
}

/// (kursi sisi kiri, kursi sisi kanan) per baris.
pub fn sisi(konfigurasi: &str) -> (u8, u8) {
    match konfigurasi {
        "1-1" => (1, 1),
        "2-1" => (2, 1),
        _ => (2, 2),
    }
}

pub const KONFIGURASI: [(&str, &str); 3] = [
    ("2-2", "2-2 · Reguler / Eksekutif"),
    ("2-1", "2-1 · VIP / Legrest"),
    ("1-1", "1-1 · Sleeper / Suite"),
];

/// Seluruh kursi untuk kapasitas & konfigurasi tertentu. Baris terakhir boleh
/// tak penuh bila kapasitas tak habis dibagi lebar baris.
pub fn layout(kapasitas: i32, konfigurasi: &str, dua_dek: bool) -> Vec<Seat> {
    let (kiri, kanan) = sisi(konfigurasi);
    let lebar = (kiri + kanan) as i32;
    let total = kapasitas.clamp(0, 200);
    let jumlah_baris = ((total + lebar - 1) / lebar).max(0) as u32;
    // Dua dek: separuh baris pertama di bawah (dibulatkan ke atas).
    let baris_dek1 = if dua_dek { jumlah_baris.div_ceil(2) } else { jumlah_baris };

    let mut out = Vec::with_capacity(total as usize);
    for i in 0..total {
        let baris = (i / lebar) as u32 + 1;
        let kolom = (i % lebar) as u8;
        let huruf = (b'A' + kolom) as char;
        out.push(Seat {
            kode: format!("{baris:02}{huruf}"),
            dek: if baris <= baris_dek1 { 1 } else { 2 },
            baris,
            kolom,
            jendela: kolom == 0 || kolom == (kiri + kanan - 1),
        });
    }
    out
}

/// Normalisasi daftar kode dari input bebas ("3a, 04B") → ["03A", "04B"].
/// Kode yang tak bisa dibaca dibuang; urutan dipertahankan, duplikat dibuang.
pub fn parse_kode_list(raw: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for part in raw.split([',', ' ', '\n', ';']) {
        let p = part.trim().to_uppercase();
        if p.is_empty() {
            continue;
        }
        let digits: String = p.chars().take_while(|c| c.is_ascii_digit()).collect();
        let rest: String = p.chars().skip(digits.len()).collect();
        let (Ok(n), 1) = (digits.parse::<u32>(), rest.chars().count()) else { continue };
        let Some(h) = rest.chars().next().filter(|c| c.is_ascii_uppercase()) else { continue };
        let kode = format!("{n:02}{h}");
        if !out.contains(&kode) {
            out.push(kode);
        }
    }
    out
}

/// Nama perangkat ramah-baca dari User-Agent, mis. "Chrome di Windows".
pub fn nama_perangkat(ua: &str) -> String {
    let browser = if ua.contains("Edg/") {
        "Edge"
    } else if ua.contains("SamsungBrowser") {
        "Samsung Internet"
    } else if ua.contains("OPR/") || ua.contains("Opera") {
        "Opera"
    } else if ua.contains("Firefox/") {
        "Firefox"
    } else if ua.contains("Chrome/") || ua.contains("CriOS/") {
        "Chrome"
    } else if ua.contains("Safari/") {
        "Safari"
    } else if ua.contains("curl/") {
        "curl"
    } else {
        "Browser"
    };
    let os = if ua.contains("iPhone") {
        "iPhone"
    } else if ua.contains("iPad") {
        "iPad"
    } else if ua.contains("Android") {
        "Android"
    } else if ua.contains("Windows") {
        "Windows"
    } else if ua.contains("Mac OS X") || ua.contains("Macintosh") {
        "macOS"
    } else if ua.contains("Linux") {
        "Linux"
    } else {
        ""
    };
    if os.is_empty() {
        browser.to_string()
    } else {
        format!("{browser} di {os}")
    }
}

/// Perangkat bergerak? (ikon ponsel vs laptop)
pub fn is_mobile(ua: &str) -> bool {
    ua.contains("Mobile") || ua.contains("Android") || ua.contains("iPhone")
}

/// "182.253.110.45" → "182.253.110.xx"; IPv6 dipotong 3 grup pertama.
pub fn samarkan_ip(ip: &str) -> String {
    if ip.is_empty() || ip == "-" {
        return "IP tak diketahui".into();
    }
    if ip.contains('.') && !ip.contains(':') {
        let parts: Vec<&str> = ip.split('.').collect();
        if parts.len() == 4 {
            return format!("{}.{}.{}.xx", parts[0], parts[1], parts[2]);
        }
    }
    let groups: Vec<&str> = ip.split(':').take(3).collect();
    format!("{}:…", groups.join(":"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn denah_2_2() {
        let s = layout(10, "2-2", false);
        assert_eq!(s.len(), 10);
        assert_eq!(s[0].kode, "01A");
        assert_eq!(s[3].kode, "01D");
        assert_eq!(s[4].kode, "02A");
        assert_eq!(s[9].kode, "03B");
        assert!(s[0].jendela && s[3].jendela && !s[1].jendela);
    }

    #[test]
    fn denah_1_1_dua_dek() {
        let s = layout(12, "1-1", true); // 6 baris → 3 bawah, 3 atas
        assert_eq!(s.iter().filter(|k| k.dek == 1).count(), 6);
        assert_eq!(s.last().unwrap().kode, "06B");
        assert_eq!(s.iter().find(|k| k.kode == "04A").unwrap().dek, 2);
        assert!(s.iter().all(|k| k.jendela));
    }

    #[test]
    fn denah_2_1_sisa() {
        let s = layout(7, "2-1", false);
        assert_eq!(s.iter().map(|k| k.kode.as_str()).collect::<Vec<_>>(), [
            "01A", "01B", "01C", "02A", "02B", "02C", "03A"
        ]);
    }

    #[test]
    fn parse_kode() {
        assert_eq!(parse_kode_list("3a, 04B 4b;x;12"), vec!["03A", "04B"]);
        assert!(parse_kode_list("").is_empty());
    }

    #[test]
    fn perangkat_dan_ip() {
        let ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/128.0 Safari/537.36";
        assert_eq!(nama_perangkat(ua), "Chrome di Windows");
        assert_eq!(
            nama_perangkat("Mozilla/5.0 (iPhone; CPU iPhone OS 17_0) AppleWebKit Version/17.0 Mobile Safari/604.1"),
            "Safari di iPhone"
        );
        assert_eq!(samarkan_ip("182.253.110.45"), "182.253.110.xx");
        assert_eq!(samarkan_ip(""), "IP tak diketahui");
    }
}
