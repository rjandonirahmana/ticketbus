use leptos::prelude::*;

#[cfg_attr(not(feature = "ssr"), allow(unused_imports))]
use super::helpers::*;
use crate::web::models::{NewRating, Rating};

#[server(CreateRating, "/api-fn")]
pub async fn create_rating(input: NewRating) -> Result<Rating, ServerFnError> {
    let claims = require_role(&["buyer"]).await?;
    let state = app_state().await?;
    state.rating_svc.rate(&claims.user_id, input).await.map_err(map_err)
}
