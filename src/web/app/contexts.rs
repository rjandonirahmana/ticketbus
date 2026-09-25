//! web/app/contexts.rs — sesi login (buyer/merchant/admin), satu-satunya
//! context global.

use leptos::prelude::*;

use crate::web::models::PublicUser;

/// `Ok(Some(user))` = sedang login sebagai `user`. Batas KEAMANAN sungguhan
/// tetap di server function (`require_role`), ini cuma dipakai UI.
pub type SessionResource = Resource<Result<Option<PublicUser>, ServerFnError>>;
