//! web/geo.rs — hitungan jarak & format untuk peta (kedua target).

/// Jarak dua titik (meter) — rumus haversine, cukup akurat untuk skala kota.
pub fn jarak_m(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    const R: f64 = 6_371_000.0;
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dp = (lat2 - lat1).to_radians();
    let dl = (lng2 - lng1).to_radians();
    let a = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    2.0 * R * a.sqrt().asin()
}

/// 350 → "350 m", 1234 → "1,2 km".
pub fn format_jarak(m: f64) -> String {
    if m < 1000.0 {
        format!("{} m", (m / 10.0).round() as i64 * 10)
    } else if m < 10_000.0 {
        format!("{:.1} km", m / 1000.0).replace('.', ",")
    } else {
        format!("{} km", (m / 1000.0).round() as i64)
    }
}

/// Perkiraan menit tempuh. Kecepatan GPS yang terlalu rendah/kosong
/// (macet, berhenti di lampu merah) dibulatkan ke 25 km/j supaya perkiraan
/// tak melonjak tak masuk akal.
pub fn eta_menit(jarak_m: f64, speed_kmh: Option<f64>) -> i64 {
    let v = speed_kmh.filter(|v| *v >= 25.0).unwrap_or(25.0);
    ((jarak_m / 1000.0) / v * 60.0).ceil().max(1.0) as i64
}

/// Koordinat masuk akal (bukan 0,0 & dalam rentang).
pub fn koordinat_valid(lat: f64, lng: f64) -> bool {
    lat.is_finite() && lng.is_finite() && (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lng)
        && !(lat == 0.0 && lng == 0.0)
}

/// Titik tengah bawaan peta bila lokasi pengguna belum diketahui (Jakarta).
pub const DEFAULT_CENTER: (f64, f64) = (-6.2088, 106.8456);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jarak_jakarta_bandung() {
        // Monas → Gedung Sate ± 116 km garis lurus.
        let d = jarak_m(-6.1754, 106.8272, -6.9025, 107.6188);
        assert!((110_000.0..125_000.0).contains(&d), "{d}");
    }

    #[test]
    fn format_dan_eta() {
        assert_eq!(format_jarak(347.0), "350 m");
        assert_eq!(format_jarak(1234.0), "1,2 km");
        assert_eq!(format_jarak(25_400.0), "25 km");
        assert_eq!(eta_menit(5000.0, Some(60.0)), 5);
        assert_eq!(eta_menit(5000.0, Some(3.0)), 12); // macet → 25 km/j
        assert!(!koordinat_valid(0.0, 0.0));
        assert!(koordinat_valid(-6.2, 106.8));
    }
}
