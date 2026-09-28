/* lajumap.js — jembatan tipis Leaflet ⇄ Leptos/WASM (lihat components/map.rs).
 *
 * Semua state peta ada di sini, dikunci per id elemen, supaya Rust cukup
 * memanggil fungsi datar (init / setUser / setItems / focus / pick / watch)
 * tanpa memegang objek JS. Aman dipanggil berulang: init() pada id yang sama
 * membuang peta lama lebih dulu (navigasi SPA memasang elemen baru).
 */
(function () {
  "use strict";
  var maps = {};

  var OSM_ATTR = '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>';

  function hasWebGL() {
    try {
      var c = document.createElement("canvas");
      return !!(c.getContext("webgl2") || c.getContext("webgl"));
    } catch (e) {
      return false;
    }
  }

  // Peta dasar, urut prioritas:
  // 1. window.LAJU_TILES.url (env CARTO_TILE_URL, disuntik app/shell.rs) —
  //    tile raster sendiri bila suatu saat pakai penyedia berbayar;
  // 2. OpenFreeMap "positron" (vektor, gratis tanpa kunci & tanpa batas,
  //    boleh komersial) lewat MapLibre GL;
  // 3. tile raster OpenStreetMap standar bila MapLibre/WebGL tak tersedia.
  // Kelas .lm-tiles memberi rona lavender (dan versi gelap) lewat CSS.
  function baseLayer() {
    if (window.LAJU_TILES && window.LAJU_TILES.url) {
      return L.tileLayer(window.LAJU_TILES.url, {
        maxZoom: 19,
        subdomains: "abcd",
        className: "lm-tiles",
        attribution: OSM_ATTR,
      });
    }
    if (window.maplibregl && L.maplibreGL && hasWebGL()) {
      try {
        return L.maplibreGL({
          style: "https://tiles.openfreemap.org/styles/positron",
          className: "lm-tiles",
          attribution: '<a href="https://openfreemap.org" target="_blank">OpenFreeMap</a> ' + OSM_ATTR,
        });
      } catch (e) {}
    }
    return L.tileLayer("https://tile.openstreetmap.org/{z}/{x}/{y}.png", {
      maxZoom: 19,
      className: "lm-tiles",
      attribution: OSM_ATTR,
    });
  }

  function esc(s) {
    return String(s == null ? "" : s).replace(/[&<>"']/g, function (c) {
      return { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c];
    });
  }

  function itemIcon(it) {
    var cls = "lm-pin lm-" + (it.kind || "bus") + (it.tone ? " lm-" + it.tone : "") + (it.active ? " lm-active" : "");
    var glyph = it.kind === "pickup" ? "location_on" : "directions_bus";
    // Titik jemput tanpa bus: label teks kapital polos seperti nama terminal di desain.
    if (it.kind === "pickup") {
      return L.divIcon({
        className: "lm-wrap",
        html: '<div class="' + cls + '"><span class="lm-dot"><span class="icon icon-fill">location_on</span></span>' +
          (it.label ? '<span class="lm-text">' + esc(it.label) + (it.badge ? " · " + esc(it.badge) : "") + "</span>" : "") +
          "</div>",
        iconSize: [0, 0],
      });
    }
    var color = /^#[0-9a-fA-F]{6}$/.test(it.color || "") ? it.color : "";
    var label = it.label
      ? '<span class="lm-label"><i></i>' + esc(it.label) +
        (it.badge ? " <b>" + esc(it.badge) + "</b>" : "") + "</span>"
      : "";
    return L.divIcon({
      className: "lm-wrap",
      html: '<div class="' + cls + '"><span class="lm-dot"' + (color && it.kind !== "pickup" ? ' style="background:' + color + '"' : "") +
        '><span class="icon icon-fill">' + glyph + "</span></span>" + label + "</div>",
      iconSize: [0, 0],
      iconAnchor: [0, 0],
    });
  }

  window.LajuMap = {
    ready: function () {
      return typeof window.L !== "undefined";
    },

    init: function (id, lat, lng, zoom, interactive) {
      if (!window.L) return false;
      var el = document.getElementById(id);
      if (!el) return false;
      if (maps[id]) {
        try { maps[id].map.remove(); } catch (e) {}
        delete maps[id];
      }
      var map = L.map(el, {
        zoomControl: false,
        attributionControl: true,
        dragging: interactive !== false,
        scrollWheelZoom: false,
        tap: true,
      }).setView([lat, lng], zoom);
      baseLayer().addTo(map);
      maps[id] = { map: map, layer: L.layerGroup().addTo(map), user: null, circle: null, pick: null, onSelect: null };
      // Elemen sering baru saja dipasang/diubah ukurannya oleh Leptos.
      setTimeout(function () { map.invalidateSize(); }, 120);
      return true;
    },

    setUser: function (id, lat, lng, radiusM, label) {
      var m = maps[id];
      if (!m) return;
      if (m.user) m.map.removeLayer(m.user);
      if (m.circle) m.map.removeLayer(m.circle);
      m.user = L.marker([lat, lng], {
        icon: L.divIcon({
          className: "lm-wrap",
          html: '<div class="lm-me"><i><span class="icon icon-fill">person_pin_circle</span></i>' +
            (label ? '<span class="lm-me-label">' + esc(label) + "</span>" : "") + "</div>",
          iconSize: [0, 0],
        }),
        zIndexOffset: 1000,
      }).addTo(m.map);
      if (radiusM > 0) {
        m.circle = L.circle([lat, lng], {
          radius: radiusM,
          stroke: false,
          fillColor: "#f59e62",
          fillOpacity: 0.22,
        }).addTo(m.map);
      }
    },

    /* items: JSON [{id, kind:"bus"|"pickup", lat, lng, label, badge, tone, active}] */
    setItems: function (id, json, fit) {
      var m = maps[id];
      if (!m) return;
      m.layer.clearLayers();
      var items = [];
      try { items = JSON.parse(json) || []; } catch (e) {}
      var pts = [];
      items.forEach(function (it) {
        // Bus live → garis putus-putus biru menuju titik jemputnya.
        if (it.to_lat != null && it.to_lng != null) {
          L.polyline([[it.lat, it.lng], [it.to_lat, it.to_lng]], {
            color: "#0f4c81", weight: 3, opacity: 0.75, dashArray: "8 8",
          }).addTo(m.layer);
          L.circleMarker([it.to_lat, it.to_lng], {
            radius: 5, color: "#fff", weight: 2, fillColor: "#0f4c81", fillOpacity: 1,
          }).addTo(m.layer);
        }
        var mk = L.marker([it.lat, it.lng], { icon: itemIcon(it), zIndexOffset: it.active ? 500 : 0 });
        mk.on("click", function () { if (m.onSelect) m.onSelect(it.id); });
        mk.addTo(m.layer);
        pts.push([it.lat, it.lng]);
      });
      if (fit) {
        if (m.user) pts.push(m.user.getLatLng());
        if (pts.length === 1) m.map.setView(pts[0], 14);
        else if (pts.length > 1) m.map.fitBounds(pts, { padding: [36, 36], maxZoom: 15 });
      }
    },

    onSelect: function (id, cb) {
      if (maps[id]) maps[id].onSelect = cb;
    },

    focus: function (id, lat, lng, zoom) {
      var m = maps[id];
      if (m) m.map.flyTo([lat, lng], zoom || 15, { duration: 0.6 });
    },

    zoom: function (id, delta) {
      var m = maps[id];
      if (m) m.map.setZoom(m.map.getZoom() + delta);
    },

    /* Pas-kan tampilan ke semua penanda (+ pengguna). */
    fitAll: function (id) {
      var m = maps[id];
      if (!m) return;
      var pts = [];
      m.layer.eachLayer(function (l) { if (l.getLatLng) pts.push(l.getLatLng()); });
      if (m.user) pts.push(m.user.getLatLng());
      if (pts.length === 1) m.map.setView(pts[0], 14);
      else if (pts.length > 1) m.map.fitBounds(pts, { padding: [40, 40], maxZoom: 15 });
    },

    invalidate: function (id) {
      var m = maps[id];
      if (m) m.map.invalidateSize();
    },

    /* Mode pemilih koordinat (form jadwal): klik peta → penanda + cb(lat, lng). */
    pick: function (id, lat, lng, hasValue, cb) {
      var m = maps[id];
      if (!m) return;
      function place(ll) {
        if (m.pick) m.map.removeLayer(m.pick);
        m.pick = L.marker(ll, { icon: itemIcon({ kind: "pickup", label: "Titik jemput" }) }).addTo(m.map);
      }
      if (hasValue) place([lat, lng]);
      m.map.on("click", function (e) {
        place(e.latlng);
        cb(e.latlng.lat, e.latlng.lng);
      });
    },

    /* Geolocation: cb(lat, lng, akurasi_m, kecepatan_kmh|-1, arah|-1), err(pesan). */
    watch: function (cb, err, high) {
      if (!navigator.geolocation) {
        err("Perangkat ini tidak mendukung lokasi");
        return -1;
      }
      return navigator.geolocation.watchPosition(
        function (p) {
          var c = p.coords;
          cb(c.latitude, c.longitude, c.accuracy || 0,
            c.speed == null ? -1 : c.speed * 3.6,
            c.heading == null || isNaN(c.heading) ? -1 : c.heading);
        },
        function (e) {
          err(e.code === 1 ? "Izin lokasi ditolak — aktifkan di pengaturan browser"
            : e.code === 3 ? "Sinyal GPS lambat, mencoba lagi…" : "Lokasi tidak tersedia");
        },
        { enableHighAccuracy: !!high, maximumAge: 5000, timeout: 20000 }
      );
    },

    /* Jaga layar tetap menyala selama driver berbagi lokasi (bila didukung). */
    wake: function (on) {
      try {
        if (on && navigator.wakeLock && !window.__lmWake) {
          navigator.wakeLock.request("screen").then(function (w) { window.__lmWake = w; }).catch(function () {});
        } else if (!on && window.__lmWake) {
          window.__lmWake.release();
          window.__lmWake = null;
        }
      } catch (e) {}
    },

    clearWatch: function (wid) {
      if (navigator.geolocation && wid >= 0) navigator.geolocation.clearWatch(wid);
    },
  };
})();
