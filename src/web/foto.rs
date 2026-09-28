//! Titik fokus foto (hasil "geser untuk atur posisi" saat unggah).
//!
//! Posisi disimpan sebagai fragmen di URL foto: `…/abc.jpg#pos=50,30`
//! (persen x,y untuk CSS `object-position`). Fragmen tak pernah dikirim ke
//! server gambar, jadi file tetap termuat apa adanya, dan tak perlu kolom DB
//! baru di setiap tabel yang menyimpan URL foto (banner, paket, bus, profil PO).

/// Pisahkan URL dari posisinya. Tanpa/rusak → posisi tengah (50, 50).
pub fn split(url: &str) -> (&str, (f64, f64)) {
    match url.split_once("#pos=") {
        Some((base, pos)) => {
            let mut it = pos.split(',').map(|v| v.trim().parse::<f64>().ok());
            match (it.next().flatten(), it.next().flatten()) {
                (Some(x), Some(y)) => (base, (clamp(x), clamp(y))),
                _ => (base, (50.0, 50.0)),
            }
        }
        None => (url, (50.0, 50.0)),
    }
}

/// URL + posisi baru. Posisi tengah tak ditulis supaya URL tetap bersih.
pub fn with_pos(url: &str, x: f64, y: f64) -> String {
    let (base, _) = split(url);
    let (x, y) = (clamp(x).round(), clamp(y).round());
    if x == 50.0 && y == 50.0 {
        base.to_string()
    } else {
        format!("{base}#pos={x},{y}")
    }
}

/// Nilai atribut `style` untuk `<img>` ber-`object-fit: cover`.
pub fn style(url: &str) -> String {
    let (_, (x, y)) = split(url);
    format!("object-position:{x}% {y}%")
}

fn clamp(v: f64) -> f64 {
    if v.is_finite() {
        v.clamp(0.0, 100.0)
    } else {
        50.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn posisi_di_fragmen() {
        assert_eq!(split("https://x/a.jpg"), ("https://x/a.jpg", (50.0, 50.0)));
        assert_eq!(split("https://x/a.jpg#pos=20,75"), ("https://x/a.jpg", (20.0, 75.0)));
        assert_eq!(split("https://x/a.jpg#pos=abc"), ("https://x/a.jpg", (50.0, 50.0)));
        assert_eq!(split("https://x/a.jpg#pos=-5,300").1, (0.0, 100.0));
        assert_eq!(with_pos("https://x/a.jpg#pos=1,1", 30.4, 60.6), "https://x/a.jpg#pos=30,61");
        assert_eq!(with_pos("https://x/a.jpg#pos=1,1", 50.0, 50.0), "https://x/a.jpg");
        assert_eq!(style("https://x/a.jpg#pos=20,75"), "object-position:20% 75%");
    }
}
