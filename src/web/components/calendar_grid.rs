use chrono::{Datelike, NaiveDate};
use leptos::prelude::*;

use crate::web::models::Schedule;

const WEEKDAYS: [&str; 7] = ["Sen", "Sel", "Rab", "Kam", "Jum", "Sab", "Min"];

fn month_weeks(year: i32, month: u32) -> Vec<Vec<Option<NaiveDate>>> {
    let Some(first) = NaiveDate::from_ymd_opt(year, month, 1) else {
        return Vec::new();
    };
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    };
    let Some(next) = next else { return Vec::new() };
    let days_in_month = (next - first).num_days();

    let lead = first.weekday().num_days_from_monday() as i64;
    let mut cells: Vec<Option<NaiveDate>> = std::iter::repeat(None).take(lead as usize).collect();
    for d in 0..days_in_month {
        cells.push(first.checked_add_signed(chrono::Duration::days(d)));
    }
    while cells.len() % 7 != 0 {
        cells.push(None);
    }
    cells.chunks(7).map(|c| c.to_vec()).collect()
}

#[component]
pub fn CalendarGrid(
    year: i32,
    month: u32,
    schedules: Vec<Schedule>,
    #[prop(into)] on_day_click: Callback<String>,
    /// Tanggal terpilih ("YYYY-MM-DD") — diberi cincin sorot.
    #[prop(optional)]
    selected: Option<String>,
) -> impl IntoView {
    let weeks = month_weeks(year, month);
    let today = super::today_wib().format("%Y-%m-%d").to_string();

    view! {
        <div class="calendar-grid">
            <div class="calendar-weekdays">
                {WEEKDAYS.iter().map(|d| view! { <div class="wd">{*d}</div> }).collect_view()}
            </div>
            <div class="calendar-body">
                {weeks
                    .into_iter()
                    .map(|week| {
                        view! {
                            <div class="calendar-row">
                                {week
                                    .into_iter()
                                    .map(|day| match day {
                                        None => view! { <div class="calendar-cell empty"></div> }.into_any(),
                                        Some(d) => {
                                            let date_str = d.format("%Y-%m-%d").to_string();
                                            let day_schedules: Vec<&Schedule> =
                                                schedules.iter().filter(|s| s.tanggal == date_str).collect();
                                            let mut cls = String::from("calendar-cell");
                                            if !day_schedules.is_empty() {
                                                cls.push_str(" has-trip");
                                            }
                                            if date_str == today {
                                                cls.push_str(" today");
                                            }
                                            if selected.as_deref() == Some(date_str.as_str()) {
                                                cls.push_str(" selected");
                                            }
                                            if date_str < today {
                                                cls.push_str(" past");
                                            }
                                            let extra = day_schedules.len().saturating_sub(3);
                                            let dots = day_schedules
                                                .iter()
                                                .take(3)
                                                .map(|s| {
                                                    let color = s.armada_color_hex.clone();
                                                    view! {
                                                        <span class="dot" style=format!("background:{color}")></span>
                                                    }
                                                })
                                                .collect_view();
                                            let click_date = date_str.clone();
                                            view! {
                                                <button
                                                    type="button"
                                                    class=cls
                                                    on:click=move |_| on_day_click.run(click_date.clone())
                                                >
                                                    <span class="daynum">{d.day()}</span>
                                                    <span class="dots">
                                                        {dots}
                                                        {(extra > 0).then(|| view! { <small>{format!("+{extra}")}</small> })}
                                                    </span>
                                                </button>
                                            }
                                                .into_any()
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
        </div>
    }
}
