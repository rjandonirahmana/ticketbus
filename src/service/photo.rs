use crate::repository::TripPhotoRepository;
use crate::web::models::TripPhoto;

#[derive(Clone)]
pub struct PhotoService {
    repo: TripPhotoRepository,
}

impl PhotoService {
    pub fn new(repo: TripPhotoRepository) -> Self {
        Self { repo }
    }

    pub async fn list(&self, armada_id: Option<&str>) -> anyhow::Result<Vec<TripPhoto>> {
        self.repo.list(armada_id).await
    }

    pub async fn list_by_merchant(&self, merchant_id: &str) -> anyhow::Result<Vec<TripPhoto>> {
        self.repo.list_by_merchant(merchant_id).await
    }

    pub async fn create(&self, armada_id: &str, url: &str, caption: &str) -> anyhow::Result<TripPhoto> {
        self.repo.create(armada_id, url, caption).await
    }

    pub async fn delete(&self, id: &str) -> anyhow::Result<()> {
        self.repo.delete(id).await
    }

    pub async fn armada_id_of(&self, id: &str) -> anyhow::Result<String> {
        self.repo.armada_id_of(id).await
    }
}
