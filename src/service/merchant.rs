//! service/merchant.rs — pendaftaran & profil Mitra PO, review admin, dan
//! halaman profil PO publik.
//!
//! Mitra baru berstatus `menunggu`: bisa login & menyiapkan armada/trayek,
//! tetapi semua daftar publik (repository/schedule.rs `PO_TAMPIL`, katalog
//! sewa, dan checkout order) menyaring PO yang belum `disetujui`.

use crate::repository::merchant::{MerchantRepository, ProfileRow};
use crate::repository::{ArmadaRepository, RouteRepository};
use crate::service::waha::WahaClient;
use crate::service::wa_message;
use crate::utils::phone;
use crate::web::models::{MerchantProfile, NewMerchantProfile, PoPage, PublicUser};

pub const STATUS: &[&str] = &["menunggu", "disetujui", "ditolak"];

#[derive(Clone)]
pub struct MerchantService {
    repo: MerchantRepository,
    armadas: ArmadaRepository,
    routes: RouteRepository,
    waha: WahaClient,
    /// Nomor CS (ADMIN_PHONE ternormalisasi) — diberi tahu ada pendaftar baru.
    cs_phone: String,
}

fn clip(s: &str, max: usize) -> String {
    s.trim().chars().take(max).collect()
}

fn url_ok(u: &str) -> bool {
    u.is_empty() || u.starts_with("https://") || u.starts_with("http://")
}

/// Rapikan & validasi input profil (dipakai daftar, ubah, dan tes).
pub fn normalize(p: NewMerchantProfile) -> anyhow::Result<(NewMerchantProfile, Option<i32>, Vec<String>)> {
    let p = NewMerchantProfile {
        nama_po: clip(&p.nama_po, 80),
        kota: clip(&p.kota, 60),
        alamat: clip(&p.alamat, 200),
        deskripsi: clip(&p.deskripsi, 600),
        tahun_berdiri: p.tahun_berdiri.trim().to_string(),
        jumlah_armada: p.jumlah_armada,
        layanan: p.layanan,
        nama_pemilik: clip(&p.nama_pemilik, 80),
        email: clip(&p.email, 120),
        nomor_izin: clip(&p.nomor_izin, 60),
        dokumen_url: p.dokumen_url.trim().to_string(),
        logo_url: p.logo_url.trim().to_string(),
        sampul_url: p.sampul_url.trim().to_string(),
    };
    if p.nama_po.chars().count() < 3 {
        anyhow::bail!("Nama PO minimal 3 huruf");
    }
    if p.kota.is_empty() {
        anyhow::bail!("Kota basis PO wajib diisi");
    }
    if !(0..=10_000).contains(&p.jumlah_armada) {
        anyhow::bail!("Jumlah armada tidak valid");
    }
    if !p.email.is_empty() && !(p.email.contains('@') && p.email.contains('.')) {
        anyhow::bail!("Format email tidak valid");
    }
    let tahun = if p.tahun_berdiri.is_empty() {
        None
    } else {
        let t: i32 = p.tahun_berdiri.parse().map_err(|_| anyhow::anyhow!("Tahun berdiri tidak valid"))?;
        if !(1900..=2100).contains(&t) {
            anyhow::bail!("Tahun berdiri tidak valid");
        }
        Some(t)
    };
    if ![&p.dokumen_url, &p.logo_url, &p.sampul_url].iter().all(|u| url_ok(u)) {
        anyhow::bail!("URL gambar tidak valid");
    }
    let mut layanan: Vec<String> = Vec::new();
    for l in p.layanan.split(',').map(|s| clip(s, 30)).filter(|s| !s.is_empty()) {
        if !layanan.contains(&l) && layanan.len() < 8 {
            layanan.push(l);
        }
    }
    Ok((p, tahun, layanan))
}

/// Profil untuk publik: kosongkan data review & kontak pribadi.
fn publik(mut p: MerchantProfile) -> MerchantProfile {
    p.nama_pemilik.clear();
    p.email.clear();
    p.nomor_izin.clear();
    p.dokumen_url.clear();
    p.catatan_admin.clear();
    p.telp.clear();
    p
}

impl MerchantService {
    pub fn new(
        repo: MerchantRepository,
        armadas: ArmadaRepository,
        routes: RouteRepository,
        waha: WahaClient,
        cs_phone: String,
    ) -> Self {
        Self { repo, armadas, routes, waha, cs_phone }
    }

    /// Validasi form /daftar-mitra SEBELUM OTP dikirim → JSON draf.
    pub fn draft_json(&self, p: NewMerchantProfile) -> anyhow::Result<String> {
        let (p, _, _) = normalize(p)?;
        Ok(serde_json::to_string(&p)?)
    }

    /// Akun mitra baru saja terverifikasi OTP → buat profil `menunggu`.
    pub async fn on_registered(&self, user: &PublicUser, draft: Option<&str>) -> anyhow::Result<()> {
        let input = draft
            .and_then(|d| serde_json::from_str::<NewMerchantProfile>(d).ok())
            .unwrap_or_else(|| NewMerchantProfile {
                nama_po: user.name.clone(),
                kota: "-".into(),
                nama_pemilik: user.name.clone(),
                ..Default::default()
            });
        let (p, tahun, layanan) = normalize(input)?;
        self.repo
            .create(&user.id, &ProfileRow { p: &p, tahun_berdiri: tahun, layanan })
            .await?;
        if !self.cs_phone.is_empty() {
            let (waha, cs) = (self.waha.clone(), self.cs_phone.clone());
            let text = wa_message::mitra_baru_ke_cs(&p.nama_po, &user.name, &user.phone, &p.kota);
            tokio::spawn(async move {
                if let Err(e) = waha.send_text(&phone::to_waha_chat_id(&cs), &text).await {
                    tracing::warn!(error = %e, "WA notifikasi mitra baru ke CS gagal");
                }
            });
        }
        Ok(())
    }

    pub async fn my_profile(&self, user_id: &str) -> anyhow::Result<Option<MerchantProfile>> {
        self.repo.get(user_id).await
    }

    pub async fn update_mine(&self, user_id: &str, p: NewMerchantProfile) -> anyhow::Result<()> {
        let (p, tahun, layanan) = normalize(p)?;
        self.repo.update(user_id, &ProfileRow { p: &p, tahun_berdiri: tahun, layanan }).await
    }

    pub async fn list(&self, status: Option<&str>) -> anyhow::Result<Vec<MerchantProfile>> {
        if let Some(s) = status {
            if !STATUS.contains(&s) {
                anyhow::bail!("Status tidak valid");
            }
        }
        self.repo.list(status).await
    }

    /// Keputusan admin + WA best-effort ke mitra.
    pub async fn decide(&self, user_id: &str, setujui: bool, catatan: &str) -> anyhow::Result<()> {
        let catatan = clip(catatan, 500);
        if !setujui && catatan.is_empty() {
            anyhow::bail!("Tulis alasan penolakan supaya mitra bisa memperbaikinya");
        }
        let status = if setujui { "disetujui" } else { "ditolak" };
        self.repo.set_status(user_id, status, &catatan).await?;
        if let Some(p) = self.repo.get(user_id).await? {
            let waha = self.waha.clone();
            let text = wa_message::mitra_diputuskan(&p.nama_po, setujui, &catatan);
            tokio::spawn(async move {
                if let Err(e) = waha.send_text(&phone::to_waha_chat_id(&p.telp), &text).await {
                    tracing::warn!(error = %e, "WA keputusan mitra gagal");
                }
            });
        }
        Ok(())
    }

    /// Halaman /po/:id — hanya PO yang sudah disetujui.
    pub async fn public_page(&self, user_id: &str) -> anyhow::Result<Option<PoPage>> {
        let Ok(uuid) = uuid::Uuid::parse_str(user_id) else {
            return Ok(None);
        };
        let id = uuid.to_string();
        let Some(profil) = self.repo.get(&id).await?.filter(|p| p.status == "disetujui") else {
            return Ok(None);
        };
        let armadas = self.armadas.list_by_merchant(&id).await?;
        let routes = self
            .routes
            .list(Some(&id))
            .await?
            .into_iter()
            .filter(|r| r.aktif)
            .map(|mut r| {
                r.driver_nama.clear();
                r.driver_telp.clear();
                r
            })
            .collect();
        let mut keberangkatan = self.repo.upcoming(&id).await?;
        for s in &mut keberangkatan {
            s.driver_telp.clear();
        }
        Ok(Some(PoPage { profil: publik(profil), armadas, routes, keberangkatan }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> NewMerchantProfile {
        NewMerchantProfile { nama_po: " PO Sinar Jaya ".into(), kota: "Solo".into(), ..Default::default() }
    }

    #[test]
    fn profil_dirapikan() {
        let (p, tahun, layanan) =
            normalize(NewMerchantProfile { layanan: "AKAP, Pariwisata,AKAP, ".into(), tahun_berdiri: "1982".into(), ..input() })
                .unwrap();
        assert_eq!(p.nama_po, "PO Sinar Jaya");
        assert_eq!(tahun, Some(1982));
        assert_eq!(layanan, vec!["AKAP", "Pariwisata"]);
    }

    #[test]
    fn profil_tak_valid() {
        assert!(normalize(NewMerchantProfile { nama_po: "PO".into(), ..input() }).is_err());
        assert!(normalize(NewMerchantProfile { kota: " ".into(), ..input() }).is_err());
        assert!(normalize(NewMerchantProfile { email: "bukan-email".into(), ..input() }).is_err());
        assert!(normalize(NewMerchantProfile { tahun_berdiri: "82".into(), ..input() }).is_err());
        assert!(normalize(NewMerchantProfile { logo_url: "javascript:x".into(), ..input() }).is_err());
    }

    #[test]
    fn publik_tanpa_data_pribadi() {
        let p = publik(MerchantProfile { telp: "628".into(), nomor_izin: "X".into(), email: "a@b.c".into(), ..Default::default() });
        assert!(p.telp.is_empty() && p.nomor_izin.is_empty() && p.email.is_empty());
    }
}
