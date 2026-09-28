//! service/storage.rs — RustFS (S3-compatible) upload untuk foto trip.
//! Pola disalin dari e-ticketing/src/service/storage.rs.

use aws_sdk_s3::{
    config::{BehaviorVersion, Builder, Credentials, Region},
    primitives::ByteStream,
    Client,
};
use uuid::Uuid;

use crate::config::config::RustFsConfig;

const MAX_SIZE: usize = 5 * 1024 * 1024;
/// Folder objek foto (trip, paket wisata, bus sewa, banner) di dalam bucket.
const FOLDER: &str = "trip-photos";

/// Validasi magic bytes — Content-Type dari client tak dipercaya begitu saja.
fn detect_image_mime(data: &[u8]) -> Option<&'static str> {
    match data {
        [0xFF, 0xD8, 0xFF, ..] => Some("image/jpeg"),
        [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, ..] => Some("image/png"),
        [0x47, 0x49, 0x46, 0x38, 0x37, 0x61, ..] => Some("image/gif"),
        [0x47, 0x49, 0x46, 0x38, 0x39, 0x61, ..] => Some("image/gif"),
        [0x52, 0x49, 0x46, 0x46, _, _, _, _, 0x57, 0x45, 0x42, 0x50, ..] => Some("image/webp"),
        _ => None,
    }
}

/// URL publik objek = `{public_url}/{bucket}/{key}` (path-style, sama dengan
/// e-ticketing & ppm). `RUSTFS_PUBLIC_URL` boleh domain saja
/// (`https://image.ulalaapi.store`) atau sudah berakhiran bucket — bucket
/// tidak ditambahkan dua kali.
fn public_base(public_url: &str, bucket: &str) -> String {
    let base = public_url.trim_end_matches('/');
    if base.ends_with(&format!("/{bucket}")) {
        base.to_string()
    } else {
        format!("{base}/{bucket}")
    }
}

#[derive(Clone)]
pub struct StorageService {
    client: Client,
    bucket: String,
    public_url: String,
}

impl StorageService {
    pub fn new(cfg: &RustFsConfig) -> Self {
        let creds = Credentials::new(&cfg.access_key, &cfg.secret_key, None, None, "rustfs");
        let config = Builder::new()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("us-east-1"))
            .endpoint_url(&cfg.endpoint)
            .credentials_provider(creds)
            .force_path_style(true)
            .build();
        Self {
            client: Client::from_conf(config),
            bucket: cfg.bucket.clone(),
            public_url: public_base(&cfg.public_url, &cfg.bucket),
        }
    }

    /// Dulu URL disimpan tanpa bucket (`{public_url}/trip-photos/…` → 404).
    /// Kembalikan (prefiks lama, prefiks benar) untuk diperbaiki di DB saat
    /// start; `None` bila konfigurasi tak pernah menghasilkan URL salah.
    pub fn legacy_prefix(cfg: &RustFsConfig) -> Option<(String, String)> {
        let lama = cfg.public_url.trim_end_matches('/');
        let benar = public_base(&cfg.public_url, &cfg.bucket);
        (lama != benar).then(|| (format!("{lama}/{FOLDER}/"), format!("{benar}/{FOLDER}/")))
    }

    pub async fn init(&self) -> anyhow::Result<()> {
        match self.client.list_buckets().send().await {
            Ok(list) => {
                let exists = list.buckets().iter().any(|b| b.name() == Some(&self.bucket));
                if exists {
                    tracing::info!(bucket = %self.bucket, "RustFS: bucket sudah ada");
                    return Ok(());
                }
                tracing::info!(bucket = %self.bucket, "RustFS: membuat bucket");
                self.client.create_bucket().bucket(&self.bucket).send().await?;
                Ok(())
            }
            Err(e) => {
                tracing::warn!(error = %e, "RustFS: gagal memeriksa bucket (lanjut jalan, upload akan gagal bila endpoint tak tersedia)");
                Ok(())
            }
        }
    }

    /// Unggah foto trip. `data` sudah dibaca penuh ke memori — cukup untuk
    /// gambar single upload berukuran wajar (dibatasi `MAX_SIZE`).
    pub async fn upload_trip_photo(&self, data: Vec<u8>) -> anyhow::Result<String> {
        if data.len() > MAX_SIZE {
            anyhow::bail!("Foto maksimal {}MB", MAX_SIZE / 1024 / 1024);
        }
        let mime = detect_image_mime(&data)
            .ok_or_else(|| anyhow::anyhow!("Format file tidak dikenali. Gunakan JPEG/PNG/WebP/GIF."))?;
        let ext = match mime {
            "image/jpeg" => "jpg",
            "image/png" => "png",
            "image/webp" => "webp",
            "image/gif" => "gif",
            _ => "bin",
        };
        let key = format!("{FOLDER}/{}.{ext}", Uuid::new_v4());

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(ByteStream::from(data))
            .content_type(mime)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Gagal unggah ke RustFS: {e}"))?;

        Ok(format!("{}/{key}", self.public_url))
    }
}

#[cfg(test)]
mod tests {
    use super::public_base;

    #[test]
    fn url_publik_memuat_bucket() {
        assert_eq!(public_base("https://image.ulalaapi.store", "lajubus"), "https://image.ulalaapi.store/lajubus");
        assert_eq!(public_base("https://image.ulalaapi.store/", "lajubus"), "https://image.ulalaapi.store/lajubus");
        assert_eq!(public_base("https://image.ulalaapi.store/lajubus", "lajubus"), "https://image.ulalaapi.store/lajubus");
    }
}
