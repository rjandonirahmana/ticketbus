//! Binding Rust → `public/lajumap.js` (Leaflet). Semua fungsi no-op di server
//! (SSR hanya merender `<div>` kosong); di browser dijaga `ready()` supaya
//! skrip peta yang gagal dimuat (CDN diblokir) tak membuat WASM panik.

#[cfg(target_arch = "wasm32")]
mod js {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(catch, js_namespace = LajuMap, js_name = ready)]
        pub fn ready() -> Result<bool, JsValue>;
        #[wasm_bindgen(catch, js_namespace = LajuMap, js_name = init)]
        pub fn init(id: &str, lat: f64, lng: f64, zoom: f64, interactive: bool) -> Result<bool, JsValue>;
        #[wasm_bindgen(catch, js_namespace = LajuMap, js_name = setUser)]
        pub fn set_user(id: &str, lat: f64, lng: f64, radius_m: f64, label: &str) -> Result<(), JsValue>;
        #[wasm_bindgen(catch, js_namespace = LajuMap, js_name = zoom)]
        pub fn zoom(id: &str, delta: f64) -> Result<(), JsValue>;
        #[wasm_bindgen(catch, js_namespace = LajuMap, js_name = fitAll)]
        pub fn fit_all(id: &str) -> Result<(), JsValue>;
        #[wasm_bindgen(catch, js_namespace = LajuMap, js_name = setItems)]
        pub fn set_items(id: &str, json: &str, fit: bool) -> Result<(), JsValue>;
        #[wasm_bindgen(catch, js_namespace = LajuMap, js_name = onSelect)]
        pub fn on_select(id: &str, cb: &js_sys::Function) -> Result<(), JsValue>;
        #[wasm_bindgen(catch, js_namespace = LajuMap, js_name = focus)]
        pub fn focus(id: &str, lat: f64, lng: f64, zoom: f64) -> Result<(), JsValue>;
        #[wasm_bindgen(catch, js_namespace = LajuMap, js_name = pick)]
        pub fn pick(id: &str, lat: f64, lng: f64, has: bool, cb: &js_sys::Function) -> Result<(), JsValue>;
        #[wasm_bindgen(catch, js_namespace = LajuMap, js_name = watch)]
        pub fn watch(cb: &js_sys::Function, err: &js_sys::Function, high: bool) -> Result<i32, JsValue>;
        #[wasm_bindgen(catch, js_namespace = LajuMap, js_name = wake)]
        pub fn wake(on: bool) -> Result<(), JsValue>;
        #[wasm_bindgen(catch, js_namespace = LajuMap, js_name = clearWatch)]
        pub fn clear_watch(wid: i32) -> Result<(), JsValue>;
    }
}

/// Satu penanda di peta.
#[derive(serde::Serialize, Clone, Debug, PartialEq)]
pub struct MapItem {
    pub id: String,
    /// "bus" | "pickup"
    pub kind: &'static str,
    pub lat: f64,
    pub lng: f64,
    pub label: String,
    pub badge: String,
    /// "live" | "stale" | "" → warna penanda.
    pub tone: &'static str,
    pub active: bool,
    /// Warna armada (#rrggbb) untuk penanda & titik label.
    pub color: String,
    /// Bus live → titik jemputnya (digambar garis putus-putus).
    pub to_lat: Option<f64>,
    pub to_lng: Option<f64>,
}

#[cfg(target_arch = "wasm32")]
fn ok() -> bool {
    js::ready().unwrap_or(false)
}

/// Pasang peta Leaflet di elemen `id`. `false` bila Leaflet belum/tidak ada.
pub fn map_init(id: &str, lat: f64, lng: f64, zoom: f64, interactive: bool) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        ok() && js::init(id, lat, lng, zoom, interactive).unwrap_or(false)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (id, lat, lng, zoom, interactive);
        false
    }
}

pub fn map_set_user(id: &str, lat: f64, lng: f64, radius_m: f64) {
    map_set_user_label(id, lat, lng, radius_m, "");
}

/// Seperti `map_set_user`, dengan label di bawah titik pengguna.
pub fn map_set_user_label(id: &str, lat: f64, lng: f64, radius_m: f64, label: &str) {
    #[cfg(target_arch = "wasm32")]
    if ok() {
        let _ = js::set_user(id, lat, lng, radius_m, label);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (id, lat, lng, radius_m, label);
}

pub fn map_zoom(id: &str, delta: f64) {
    #[cfg(target_arch = "wasm32")]
    if ok() {
        let _ = js::zoom(id, delta);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (id, delta);
}

pub fn map_fit_all(id: &str) {
    #[cfg(target_arch = "wasm32")]
    if ok() {
        let _ = js::fit_all(id);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = id;
}

pub fn map_set_items(id: &str, items: &[MapItem], fit: bool) {
    #[cfg(target_arch = "wasm32")]
    if ok() {
        let json = serde_json::to_string(items).unwrap_or_else(|_| "[]".into());
        let _ = js::set_items(id, &json, fit);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (id, items, fit);
}

pub fn map_focus(id: &str, lat: f64, lng: f64, zoom: f64) {
    #[cfg(target_arch = "wasm32")]
    if ok() {
        let _ = js::focus(id, lat, lng, zoom);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (id, lat, lng, zoom);
}

/// Klik penanda → `cb(id_item)`. Closure sengaja dibocorkan (`forget`): umurnya
/// = umur peta, dan init() berikutnya menggantinya.
pub fn map_on_select(id: &str, cb: impl Fn(String) + 'static) {
    #[cfg(target_arch = "wasm32")]
    if ok() {
        use wasm_bindgen::prelude::*;
        let c = Closure::<dyn Fn(String)>::new(cb);
        let _ = js::on_select(id, c.as_ref().unchecked_ref());
        c.forget();
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (id, cb);
}

/// Mode pemilih koordinat: klik peta → `cb(lat, lng)`.
pub fn map_pick(id: &str, current: Option<(f64, f64)>, cb: impl Fn(f64, f64) + 'static) {
    #[cfg(target_arch = "wasm32")]
    if ok() {
        use wasm_bindgen::prelude::*;
        let c = Closure::<dyn Fn(f64, f64)>::new(cb);
        let (lat, lng) = current.unwrap_or((0.0, 0.0));
        let _ = js::pick(id, lat, lng, current.is_some(), c.as_ref().unchecked_ref());
        c.forget();
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (id, current, cb);
}

/// Pantau lokasi perangkat. `cb(lat, lng, akurasi, kecepatan_kmh, arah)` —
/// kecepatan/arah bernilai negatif bila tak diketahui. Mengembalikan id watch
/// (-1 bila tak didukung).
pub fn geo_watch(
    cb: impl Fn(f64, f64, f64, f64, f64) + 'static,
    err: impl Fn(String) + 'static,
    high_accuracy: bool,
) -> i32 {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::prelude::*;
        if !ok() {
            err("Modul peta belum dimuat".into());
            return -1;
        }
        let c = Closure::<dyn Fn(f64, f64, f64, f64, f64)>::new(cb);
        let e = Closure::<dyn Fn(String)>::new(err);
        let wid = js::watch(c.as_ref().unchecked_ref(), e.as_ref().unchecked_ref(), high_accuracy).unwrap_or(-1);
        c.forget();
        e.forget();
        wid
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (cb, err, high_accuracy);
        -1
    }
}

pub fn geo_clear(wid: i32) {
    #[cfg(target_arch = "wasm32")]
    if ok() {
        let _ = js::clear_watch(wid);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = wid;
}

/// Jaga layar tetap menyala (Wake Lock API) — untuk HP driver.
pub fn keep_awake(on: bool) {
    #[cfg(target_arch = "wasm32")]
    if ok() {
        let _ = js::wake(on);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = on;
}

/// Waktu sekarang (ms) di browser; 0 di server.
pub fn now_ms() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        0.0
    }
}
