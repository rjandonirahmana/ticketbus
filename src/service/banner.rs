//! service/banner.rs — banner promo beranda. Hanya admin yang mengelola
//! (dicek di server fn); service ini merapikan & memvalidasi input.

use chrono::NaiveDate;

use crate::repository::banner::BannerRepository;
use crate::web::models::{Banner, NewBanner};

pub const TEMA: &[&str] = &["sapphire", "malam", "emerald", "tangerine"];

#[derive(Clone)]
pub struct BannerService {
    repo: BannerRepository,
}

/// Tanggal hari ini menurut WIB — sama dengan yang dipakai beranda.
fn today_wib() -> NaiveDate {
    (chrono::Utc::now() + chrono::Duration::hours(7)).date_naive()
}

fn parse_date(s: &str) -> anyhow::Result<Option<NaiveDate>> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(None);
    }
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map(Some)
        .map_err(|_| anyhow::anyhow!("Format tanggal tidak valid"))
}

/// Tautan banner: path internal ("/wisata", bukan "//host") atau http(s).
fn valid_link(url: &str) -> bool {
    url.is_empty()
        || (url.starts_with('/') && !url.starts_with("//"))
        || url.starts_with("https://")
        || url.starts_with("http://")
}

fn clip(s: &str, max: usize) -> String {
    s.trim().chars().take(max).collect()
}

/// Rapikan & validasi input form. Mengembalikan (input bersih, mulai, selesai).
pub fn normalize(b: NewBanner) -> anyhow::Result<(NewBanner, Option<NaiveDate>, Option<NaiveDate>)> {
    let b = NewBanner {
        judul: clip(&b.judul, 80),
        subjudul: clip(&b.subjudul, 140),
        label: clip(&b.label, 32),
        kode_promo: clip(&b.kode_promo, 24).to_uppercase(),
        cta_label: clip(&b.cta_label, 24),
        link_url: b.link_url.trim().to_string(),
        gambar_url: b.gambar_url.trim().to_string(),
        tema: b.tema,
        mulai: b.mulai,
        selesai: b.selesai,
        urutan: b.urutan.clamp(0, 999),
    };
    if b.judul.is_empty() && b.gambar_url.is_empty() {
        anyhow::bail!("Isi judul atau unggah gambar banner");
    }
    if !TEMA.contains(&b.tema.as_str()) {
        anyhow::bail!("Tema tidak valid");
    }
    if !valid_link(&b.link_url) {
        anyhow::bail!("Tautan harus diawali \"/\" (halaman LajuBus) atau https://");
    }
    if !(b.gambar_url.is_empty() || b.gambar_url.starts_with("https://") || b.gambar_url.starts_with("http://")) {
        anyhow::bail!("URL gambar tidak valid");
    }
    let mulai = parse_date(&b.mulai)?;
    let selesai = parse_date(&b.selesai)?;
    if let (Some(m), Some(s)) = (mulai, selesai) {
        if s < m {
            anyhow::bail!("Tanggal selesai tidak boleh sebelum tanggal mulai");
        }
    }
    Ok((b, mulai, selesai))
}

impl BannerService {
    pub fn new(repo: BannerRepository) -> Self {
        Self { repo }
    }

    pub async fn live(&self) -> anyhow::Result<Vec<Banner>> {
        self.repo.list_live(today_wib()).await
    }

    pub async fn all(&self) -> anyhow::Result<Vec<Banner>> {
        self.repo.list_all().await
    }

    /// `id = None` → banner baru; selain itu ubah banner tersebut.
    pub async fn save(&self, id: Option<&str>, input: NewBanner) -> anyhow::Result<String> {
        let (b, mulai, selesai) = normalize(input)?;
        match id {
            Some(id) => {
                self.repo.update(id, &b, mulai, selesai).await?;
                Ok(id.to_string())
            }
            None => self.repo.create(&b, mulai, selesai).await,
        }
    }

    pub async fn set_active(&self, id: &str, aktif: bool) -> anyhow::Result<()> {
        self.repo.set_active(id, aktif).await
    }

    pub async fn release_now(&self, id: &str) -> anyhow::Result<()> {
        self.repo.release_now(id, today_wib()).await
    }

    pub async fn delete(&self, id: &str) -> anyhow::Result<()> {
        self.repo.delete(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web::models::BannerStatus;

    fn input() -> NewBanner {
        NewBanner { judul: " Diskon 25% ".into(), tema: "sapphire".into(), ..Default::default() }
    }

    #[test]
    fn tautan_hanya_internal_atau_http() {
        assert!(valid_link(""));
        assert!(valid_link("/wisata"));
        assert!(valid_link("https://contoh.id/promo"));
        assert!(!valid_link("//evil.com"));
        assert!(!valid_link("javascript:alert(1)"));
    }

    #[test]
    fn normalisasi_input() {
        let (b, m, s) = normalize(NewBanner { kode_promo: " lajuseru ".into(), ..input() }).unwrap();
        assert_eq!(b.judul, "Diskon 25%");
        assert_eq!(b.kode_promo, "LAJUSERU");
        assert_eq!((m, s), (None, None));
        assert!(normalize(NewBanner { judul: " ".into(), ..input() }).is_err());
        assert!(normalize(NewBanner { judul: String::new(), gambar_url: "https://x/y.jpg".into(), ..input() }).is_ok());
        assert!(normalize(NewBanner { tema: "hijau".into(), ..input() }).is_err());
        assert!(normalize(NewBanner { mulai: "2026-10-05".into(), selesai: "2026-10-01".into(), ..input() }).is_err());
    }

    #[test]
    fn status_tayang() {
        let b = |aktif: bool, mulai: &str, selesai: &str| Banner {
            id: String::new(),
            judul: String::new(),
            subjudul: String::new(),
            label: String::new(),
            kode_promo: String::new(),
            cta_label: String::new(),
            link_url: String::new(),
            gambar_url: String::new(),
            tema: String::new(),
            mulai: mulai.into(),
            selesai: selesai.into(),
            urutan: 0,
            aktif,
        };
        let today = "2026-09-28";
        assert_eq!(b(true, "", "").status(today), BannerStatus::Tayang);
        assert_eq!(b(true, "2026-09-28", "2026-09-28").status(today), BannerStatus::Tayang);
        assert_eq!(b(true, "2026-10-01", "").status(today), BannerStatus::Terjadwal);
        assert_eq!(b(true, "", "2026-09-27").status(today), BannerStatus::Berakhir);
        assert_eq!(b(false, "", "").status(today), BannerStatus::Draf);
    }
}
