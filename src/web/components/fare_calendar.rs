//! "Kalender Tarif & Kursi" — jendela 2 minggu (Senin–Minggu) berisi tarif
//! termurah per tanggal, diwarnai: hemat / normal / padat-weekend / penuh.

use chrono::{Datelike, Duration, NaiveDate, Weekday};
use leptos::prelude::*;

use super::Icon;
use crate::web::models::Schedule;

const WEEKDAYS: [&str; 7] = ["SEN", "SEL", "RAB", "KAM", "JUM", "SAB", "MIN"];

/// 210000 → "210k", 1250000 → "1,2jt".
pub fn harga_singkat(n: i64) -> String {
    if n >= 1_000_000 {
        let jt = n as f64 / 1_000_000.0;
        let s = format!("{jt:.1}").replace('.', ",");
        format!("{}jt", s.trim_end_matches(",0"))
    } else if n >= 1_000 {
        format!("{}k", n / 1_000)
    } else {
        n.to_string()
    }
}

/// Senin pada minggu yang memuat `d`.
pub fn week_start(d: NaiveDate) -> NaiveDate {
    d - Duration::days(d.weekday().num_days_from_monday() as i64)
}

/// Tarif termurah (dari jadwal yang masih punya kursi) pada `date`.
pub fn min_fare(schedules: &[Schedule], date: &str) -> Option<i64> {
    schedules
        .iter()
        .filter(|s| s.tanggal == date && s.sisa_kursi() > 0)
        .map(|s| s.harga)
        .min()
}

#[derive(Clone, Copy, PartialEq)]
enum Tier {
    Kosong,
    Hemat,
    Normal,
    Padat,
    Penuh,
}

fn tier(schedules: &[Schedule], date: NaiveDate, window_min: Option<i64>) -> Tier {
    let ds = date.format("%Y-%m-%d").to_string();
    let day: Vec<&Schedule> = schedules.iter().filter(|s| s.tanggal == ds).collect();
    if day.is_empty() {
        return Tier::Kosong;
    }
    let kapasitas: i32 = day.iter().map(|s| s.kapasitas).sum();
    let terjual: i32 = day.iter().map(|s| s.kursi_terjual).sum();
    if day.iter().all(|s| s.sisa_kursi() <= 0) {
        return Tier::Penuh;
    }
    let weekend = matches!(date.weekday(), Weekday::Sat | Weekday::Sun);
    if weekend || (kapasitas > 0 && terjual * 100 / kapasitas >= 75) {
        return Tier::Padat;
    }
    if min_fare(schedules, &ds).is_some() && min_fare(schedules, &ds) == window_min {
        Tier::Hemat
    } else {
        Tier::Normal
    }
}

#[component]
pub fn FareCalendar(
    /// Senin pertama jendela 2 minggu.
    start: NaiveDate,
    schedules: Vec<Schedule>,
    #[prop(into)] selected: String,
    #[prop(into)] today: String,
    #[prop(into)] on_pick: Callback<String>,
    #[prop(into)] on_shift: Callback<i64>,
) -> impl IntoView {
    let days: Vec<NaiveDate> = (0..14).map(|i| start + Duration::days(i)).collect();
    let window_min = days
        .iter()
        .filter(|d| d.format("%Y-%m-%d").to_string() >= today)
        .filter_map(|d| min_fare(&schedules, &d.format("%Y-%m-%d").to_string()))
        .min();
    let title = {
        // Judul = bulan dari tanggal terpilih bila ada di jendela, selain itu awal jendela.
        let anchor = NaiveDate::parse_from_str(&selected, "%Y-%m-%d")
            .ok()
            .filter(|d| days.contains(d))
            .unwrap_or(start);
        super::month_label(anchor.year(), anchor.month())
    };

    view! {
        <div class="card fare-cal">
            <div class="fare-head">
                <div>
                    <span class="label-caps fare-kicker">"Kalender Tarif & Kursi"</span>
                    <h2>
                        {title}
                        <Icon name="calendar_month" />
                    </h2>
                </div>
                <div class="fare-nav">
                    <button type="button" title="2 minggu sebelumnya" on:click=move |_| on_shift.run(-14)>
                        <Icon name="chevron_left" />
                    </button>
                    <button type="button" title="2 minggu berikutnya" on:click=move |_| on_shift.run(14)>
                        <Icon name="chevron_right" />
                    </button>
                </div>
            </div>
            <div class="fare-weekdays">
                {WEEKDAYS
                    .iter()
                    .enumerate()
                    .map(|(i, d)| {
                        let cls = match i {
                            5 => "wd-sat",
                            6 => "wd-sun",
                            _ => "",
                        };
                        view! { <span class=cls>{*d}</span> }
                    })
                    .collect_view()}
            </div>
            <div class="fare-grid">
                {days
                    .into_iter()
                    .map(|d| {
                        let ds = d.format("%Y-%m-%d").to_string();
                        let past = ds < today;
                        let is_sel = ds == selected;
                        let is_today = ds == today;
                        let t = tier(&schedules, d, window_min);
                        let fare = min_fare(&schedules, &ds);
                        let price = match (fare, t) {
                            (Some(f), _) => harga_singkat(f),
                            (None, Tier::Penuh) => "Penuh".to_string(),
                            _ => "–".to_string(),
                        };
                        let mut cls = String::from("fare-cell");
                        cls.push_str(match t {
                            Tier::Kosong => " t-empty",
                            Tier::Hemat => " t-hemat",
                            Tier::Normal => " t-normal",
                            Tier::Padat => " t-padat",
                            Tier::Penuh => " t-penuh",
                        });
                        if past {
                            cls.push_str(" past");
                        }
                        if is_sel {
                            cls.push_str(" selected");
                        }
                        if is_today {
                            cls.push_str(" today");
                        }
                        let pick = ds.clone();
                        view! {
                            <button type="button" class=cls on:click=move |_| on_pick.run(pick.clone())>
                                {if is_sel {
                                    Some(view! { <span class="fare-tag">"Pilih"</span> })
                                } else {
                                    None
                                }}
                                {(!is_sel && is_today).then(|| view! { <span class="fare-tag fare-tag-today">"Hari ini"</span> })}
                                <span class="fare-day">{d.day()}</span>
                                <span class="fare-price">{price}</span>
                            </button>
                        }
                    })
                    .collect_view()}
            </div>
            <div class="fare-legend">
                <span>
                    <i class="lg-hemat"></i>
                    "Tarif Hemat"
                </span>
                <span>
                    <i class="lg-normal"></i>
                    "Normal"
                </span>
                <span>
                    <i class="lg-padat"></i>
                    "Padat / Weekend"
                </span>
                <span>
                    <i class="lg-penuh"></i>
                    "Penuh"
                </span>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harga_ringkas() {
        assert_eq!(harga_singkat(210_000), "210k");
        assert_eq!(harga_singkat(1_250_000), "1,2jt");
        assert_eq!(harga_singkat(2_000_000), "2jt");
        assert_eq!(harga_singkat(500), "500");
    }

    #[test]
    fn awal_minggu_senin() {
        let jum = NaiveDate::from_ymd_opt(2026, 9, 25).unwrap();
        assert_eq!(week_start(jum), NaiveDate::from_ymd_opt(2026, 9, 21).unwrap());
    }
}
