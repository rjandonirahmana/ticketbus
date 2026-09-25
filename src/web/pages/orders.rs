use leptos::prelude::*;

use crate::web::api::list_my_orders;
use crate::web::components::{format_rupiah, format_tanggal, Icon, PageHead};

#[component]
pub fn OrdersPage() -> impl IntoView {
    let orders = Resource::new(|| (), |_| list_my_orders());

    view! {
        <div class="page">
            <PageHead title="Tiket Saya" subtitle="Riwayat pemesanan & e-tiket perjalanan" />
            <Suspense fallback=|| view! { <div class="skeleton-card"></div> }>
                {move || {
                    let list = orders.get().and_then(|r| r.ok()).unwrap_or_default();
                    if list.is_empty() {
                        view! {
                            <div class="card empty-state">
                                <Icon name="confirmation_number" />
                                <p>"Belum ada tiket. Yuk cari jadwal perjalananmu!"</p>
                                <a href="/" class="btn btn-cta">
                                    <Icon name="search" />
                                    "Cari Jadwal Bis"
                                </a>
                            </div>
                        }
                            .into_any()
                    } else {
                        view! {
                            <div class="ticket-list">
                                {list
                                    .into_iter()
                                    .map(|o| {
                                        let href = format!("/orders/{}", o.id);
                                        view! {
                                            <a class="ticket" href=href>
                                                <div class="ticket-main">
                                                    <div class="ticket-top">
                                                        <span class="label-caps">{o.armada_name.clone()}</span>
                                                        <span class="pill pill-ok">
                                                            <Icon name="check_circle" />
                                                            {if o.status == "paid" { "Lunas".to_string() } else { o.status.clone() }}
                                                        </span>
                                                    </div>
                                                    <h3>{o.tujuan.clone()}</h3>
                                                    <p>
                                                        <Icon name="calendar_today" />
                                                        {format_tanggal(&o.tanggal)}
                                                        <span class="dot-sep"></span>
                                                        <Icon name="event_seat" />
                                                        {format!("{} kursi", o.jumlah_tiket)}
                                                    </p>
                                                </div>
                                                <div class="ticket-stub">
                                                    <span class="order-code">{o.kode_order.clone()}</span>
                                                    <strong class="price-md">{format!("Rp {}", format_rupiah(o.total_harga))}</strong>
                                                    {if o.sudah_dirating {
                                                        view! {
                                                            <span class="stub-note">
                                                                <Icon name="star" filled=true />
                                                                "Sudah dirating"
                                                            </span>
                                                        }
                                                            .into_any()
                                                    } else {
                                                        view! {
                                                            <span class="stub-note accent">
                                                                "Beri rating"
                                                                <Icon name="chevron_right" />
                                                            </span>
                                                        }
                                                            .into_any()
                                                    }}
                                                </div>
                                            </a>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        }
                            .into_any()
                    }
                }}
            </Suspense>
        </div>
    }
}
