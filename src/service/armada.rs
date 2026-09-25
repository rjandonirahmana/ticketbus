use crate::repository::ArmadaRepository;
use crate::service::auth::Claims;
use crate::web::models::Armada;

#[derive(Clone)]
pub struct ArmadaService {
    repo: ArmadaRepository,
}

impl ArmadaService {
    pub fn new(repo: ArmadaRepository) -> Self {
        Self { repo }
    }

    pub async fn list(&self) -> anyhow::Result<Vec<Armada>> {
        self.repo.list().await
    }

    pub async fn list_by_merchant(&self, merchant_id: &str) -> anyhow::Result<Vec<Armada>> {
        self.repo.list_by_merchant(merchant_id).await
    }

    /// Admin → armada platform (`merchant_id = NULL`). Merchant → armada
    /// miliknya sendiri. Peran lain harus sudah ditolak lebih dulu oleh
    /// `require_role` di server fn.
    pub async fn create(&self, actor: &Claims, name: &str, color_hex: &str) -> anyhow::Result<Armada> {
        let name = name.trim();
        if name.is_empty() {
            anyhow::bail!("Nama armada tidak boleh kosong");
        }
        if !valid_hex(color_hex) {
            anyhow::bail!("Warna harus format hex, mis. #2563eb");
        }
        let merchant_id = match actor.role.as_str() {
            "admin" => None,
            "merchant" => Some(actor.user_id.as_str()),
            _ => anyhow::bail!("Peran tidak diizinkan membuat armada"),
        };
        self.repo.create(name, color_hex, merchant_id).await
    }

    pub async fn update(&self, actor: &Claims, id: &str, name: &str, color_hex: &str) -> anyhow::Result<()> {
        let name = name.trim();
        if name.is_empty() {
            anyhow::bail!("Nama armada tidak boleh kosong");
        }
        if !valid_hex(color_hex) {
            anyhow::bail!("Warna harus format hex, mis. #2563eb");
        }
        self.check_ownership(actor, id).await?;
        self.repo.update(id, name, color_hex).await
    }

    pub async fn delete(&self, actor: &Claims, id: &str) -> anyhow::Result<()> {
        self.check_ownership(actor, id).await?;
        self.repo.delete(id).await
    }

    /// Dipakai juga oleh gerbang upload foto trip (`web/api/upload.rs`).
    pub async fn check_ownership(&self, actor: &Claims, armada_id: &str) -> anyhow::Result<()> {
        if actor.role == "admin" {
            return Ok(());
        }
        let owner = self.repo.merchant_id_of(armada_id).await?;
        if owner.as_deref() == Some(actor.user_id.as_str()) {
            return Ok(());
        }
        anyhow::bail!("Anda tidak berhak mengubah armada ini")
    }
}

fn valid_hex(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}
