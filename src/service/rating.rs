use crate::repository::{OrderRepository, RatingRepository};
use crate::web::models::{NewRating, Rating};

#[derive(Clone)]
pub struct RatingService {
    repo: RatingRepository,
    orders: OrderRepository,
}

impl RatingService {
    pub fn new(repo: RatingRepository, orders: OrderRepository) -> Self {
        Self { repo, orders }
    }

    /// Rating hanya boleh untuk order MILIK pembeli itu sendiri (dijaga oleh
    /// `OrderRepository::get_detail` yang mem-filter `buyer_id`), dan hanya
    /// sekali per order (dicek di sini untuk pesan yang jelas, ditegakkan
    /// ulang oleh `UNIQUE(order_id)` di database sebagai jaring terakhir).
    pub async fn rate(&self, buyer_id: &str, input: NewRating) -> anyhow::Result<Rating> {
        let order = self
            .orders
            .get_detail(&input.order_id, buyer_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Order tidak ditemukan"))?;
        if order.sudah_dirating {
            anyhow::bail!("Order ini sudah pernah diberi rating");
        }
        self.repo.create(buyer_id, &order.armada_id, &input).await
    }
}
