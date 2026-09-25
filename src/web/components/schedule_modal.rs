use leptos::prelude::*;

use super::{format_tanggal, Icon, ScheduleCard};
use crate::web::models::Schedule;

#[component]
pub fn ScheduleModal(
    date: String,
    schedules: Vec<Schedule>,
    is_admin: bool,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] on_delete: Callback<String>,
) -> impl IntoView {
    let count = schedules.len();
    view! {
        <div class="modal-overlay" on:click=move |_| on_close.run(())>
            <div class="sheet" on:click=|ev| ev.stop_propagation()>
                <div class="sheet-grip"></div>
                <div class="sheet-header">
                    <div>
                        <span class="label-caps">"Jadwal Tanggal"</span>
                        <h3>{format_tanggal(&date)}</h3>
                    </div>
                    <span class="pill pill-primary">{format!("{count} keberangkatan")}</span>
                    <button type="button" class="icon-btn" title="Tutup" on:click=move |_| on_close.run(())>
                        <Icon name="close" />
                    </button>
                </div>
                <div class="sheet-body">
                    {if schedules.is_empty() {
                        view! {
                            <div class="empty-state">
                                <Icon name="event_busy" />
                                <p>"Tidak ada jadwal pada tanggal ini."</p>
                            </div>
                        }
                            .into_any()
                    } else {
                        schedules
                            .into_iter()
                            .map(|s| {
                                if is_admin {
                                    view! { <ScheduleCard schedule=s on_delete=on_delete /> }.into_any()
                                } else {
                                    view! { <ScheduleCard schedule=s /> }.into_any()
                                }
                            })
                            .collect_view()
                            .into_any()
                    }}
                </div>
            </div>
        </div>
    }
}
