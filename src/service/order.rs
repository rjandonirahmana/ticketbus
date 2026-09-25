use crate::repository::OrderRepository;
use crate::web::models::{NewOrder, OrderDetail, OrderSummary};

#[derive(Clone)]
pub struct OrderService {
    repo: OrderRepository,
}

impl OrderService {
    pub fn new(repo: OrderRepository) -> Self {
        Self { repo }
    }

    pub async fn checkout(&self, buyer_id: &str, input: NewOrder) -> anyhow::Result<OrderDetail> {
        if input.nama_pemesan.trim().is_empty() {
            anyhow::bail!("Nama pemesan wajib diisi");
        }
        if input.telp_pemesan.trim().is_empty() {
            anyhow::bail!("No. HP pemesan wajib diisi");
        }
        self.repo.create(buyer_id, &input).await
    }

    pub async fn list_by_buyer(&self, buyer_id: &str) -> anyhow::Result<Vec<OrderSummary>> {
        self.repo.list_by_buyer(buyer_id).await
    }

    pub async fn get_detail(&self, order_id: &str, buyer_id: &str) -> anyhow::Result<Option<OrderDetail>> {
        self.repo.get_detail(order_id, buyer_id).await
    }
}
