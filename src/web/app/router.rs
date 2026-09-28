//! web/app/router.rs — root `App` component + tabel rute.

use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::{
    components::{FlatRoutes, Route, Router},
    path,
};

use crate::web::pages::{
    AccountPage, AdminPage, BrowsePage, BusMapPage, DriverPage, ChangePasswordPage, ForgotPasswordPage, LoginPage, MerchantPage, MerchantRegisterPage, NotFoundPage, OrderDetailPage, OrdersPage, PoPublicPage, RegisterPage,
    HelpPage, SeatSelectPage, SecurityPage, SewaPage, VerifyOtpPage, WisataPage,
};

use crate::web::components::{AppBar, BottomNav};

use super::guards::{AdminGuard, AnyUserGuard, BuyerGuard, MerchantGuard};
use super::providers::provide_all_app_contexts;

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    provide_all_app_contexts();

    view! {
        <Title text="LajuBus — Tiket & Jadwal Bis Antarkota" />
        <Meta name="description" content="LajuBus: cari jadwal bis antarkota, pilih kursi, dan beli tiket resmi dari mitra PO." />

        <Router>
            <AppBar />
            <main class="app-shell">
                <FlatRoutes fallback=NotFoundPage>
                    <Route path=path!("/") view=BrowsePage />
                    <Route path=path!("/sewa") view=SewaPage />
                    <Route path=path!("/pesan/:id") view=SeatSelectPage />
                    <Route path=path!("/bantuan") view=HelpPage />
                    <Route path=path!("/peta") view=BusMapPage />
                    <Route path=path!("/driver/:token") view=DriverPage />
                    <Route path=path!("/wisata") view=WisataPage />
                    <Route path=path!("/login") view=LoginPage />
                    <Route path=path!("/register") view=RegisterPage />
                    <Route path=path!("/daftar-mitra") view=MerchantRegisterPage />
                    <Route path=path!("/po/:id") view=PoPublicPage />
                    <Route path=path!("/verify-otp") view=VerifyOtpPage />
                    <Route path=path!("/lupa-password") view=ForgotPasswordPage />
                    <Route
                        path=path!("/akun")
                        view=|| view! { <AnyUserGuard><AccountPage /></AnyUserGuard> }
                    />
                    <Route
                        path=path!("/akun/keamanan")
                        view=|| view! { <AnyUserGuard><SecurityPage /></AnyUserGuard> }
                    />
                    <Route
                        path=path!("/akun/password")
                        view=|| view! { <AnyUserGuard><ChangePasswordPage /></AnyUserGuard> }
                    />
                    <Route
                        path=path!("/orders")
                        view=|| view! { <BuyerGuard><OrdersPage /></BuyerGuard> }
                    />
                    <Route
                        path=path!("/orders/:id")
                        view=|| view! { <BuyerGuard><OrderDetailPage /></BuyerGuard> }
                    />
                    <Route
                        path=path!("/merchant")
                        view=|| view! { <MerchantGuard><MerchantPage /></MerchantGuard> }
                    />
                    <Route
                        path=path!("/admin")
                        view=|| view! { <AdminGuard><AdminPage /></AdminGuard> }
                    />
                </FlatRoutes>
            </main>
            <BottomNav />
        </Router>
    }
}
