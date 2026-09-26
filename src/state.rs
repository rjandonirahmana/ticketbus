use deadpool_postgres::Pool;

use crate::service::{
    ArmadaService, AuthService, OrderService, PhotoService, RatingService, RentalService, ScheduleService, StorageService,
};

pub struct AppState {
    pub pool: Pool,
    /// Pembatas laju in-memory bersama (juga dipakai AuthService & RentalService).
    pub rate: std::sync::Arc<crate::service::RateLimiter>,
    pub armada_svc: ArmadaService,
    pub schedule_svc: ScheduleService,
    pub photo_svc: PhotoService,
    pub order_svc: OrderService,
    pub rating_svc: RatingService,
    pub rental_svc: RentalService,
    pub auth_svc: AuthService,
    pub storage: StorageService,
}
