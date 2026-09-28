//! Jam perjalanan "HH:MM" (WIB) & hari operasi trayek — dipakai server
//! (validasi trayek) dan UI (durasi, "+1 hari", chip hari) supaya aturannya
//! satu sumber.

/// "17:00" / "7.30" / "7:5" → menit sejak 00:00. `None` bila tak valid.
pub fn menit(jam: &str) -> Option<u32> {
    let mut it = jam.trim().split([':', '.']);
    let h: u32 = it.next()?.trim().parse().ok()?;
    let m: u32 = it.next().map(|m| m.trim().parse().ok()).unwrap_or(Some(0))?;
    if it.next().is_some() || h > 23 || m > 59 {
        return None;
    }
    Some(h * 60 + m)
}

/// Rapikan ke "HH:MM" (`None` bila tak valid).
pub fn normalisasi(jam: &str) -> Option<String> {
    menit(jam).map(|t| format!("{:02}:{:02}", t / 60, t % 60))
}

/// Lama perjalanan dalam menit + apakah tiba esok hari. Jam tiba yang sama
/// dengan / lebih awal dari jam berangkat dianggap esok hari.
pub fn durasi(berangkat: &str, tiba: &str) -> Option<(u32, bool)> {
    let (b, t) = (menit(berangkat)?, menit(tiba)?);
    if t > b {
        Some((t - b, false))
    } else {
        Some((t + 24 * 60 - b, true))
    }
}

/// 720 → "12j", 735 → "12j 15m".
pub fn durasi_label(menit: u32) -> String {
    match (menit / 60, menit % 60) {
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}j"),
        (h, m) => format!("{h}j {m}m"),
    }
}

/// Jadwal trayek tetap dibuka untuk dipesan sejauh ini ke depan.
pub const HORIZON_HARI: i32 = 30;

pub const HARI: [&str; 7] = ["Sen", "Sel", "Rab", "Kam", "Jum", "Sab", "Min"];
pub const SEMUA_HARI: i16 = 127;

/// Apakah trayek beroperasi pada hari ke-`idx` (0 = Senin).
pub fn hari_aktif(mask: i16, idx: u32) -> bool {
    mask & (1 << idx) != 0
}

/// 127 → "Setiap hari", 31 → "Sen, Sel, Rab, Kam, Jum".
pub fn hari_label(mask: i16) -> String {
    if mask & SEMUA_HARI == SEMUA_HARI {
        return "Setiap hari".into();
    }
    HARI.iter()
        .enumerate()
        .filter(|(i, _)| hari_aktif(mask, *i as u32))
        .map(|(_, h)| *h)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jam_dan_durasi() {
        assert_eq!(menit("17:00"), Some(1020));
        assert_eq!(normalisasi("7.5").as_deref(), Some("07:05"));
        assert_eq!(normalisasi("7").as_deref(), Some("07:00"));
        assert_eq!(menit("24:00"), None);
        assert_eq!(menit("abc"), None);
        // Solo 17:00 → Jakarta 05:00 esok hari = 12 jam.
        assert_eq!(durasi("17:00", "05:00"), Some((720, true)));
        assert_eq!(durasi("08:00", "15:30"), Some((450, false)));
        assert_eq!(durasi_label(735), "12j 15m");
        assert_eq!(durasi_label(720), "12j");
    }

    #[test]
    fn hari_operasi() {
        assert_eq!(hari_label(127), "Setiap hari");
        assert_eq!(hari_label(0b0011111), "Sen, Sel, Rab, Kam, Jum");
        assert!(hari_aktif(0b1000000, 6));
        assert!(!hari_aktif(0b1000000, 0));
    }
}
