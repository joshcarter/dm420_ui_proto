//! Martian FT8 console — exact layout, palette, and fake-data tables.
//!
//! Pure data + small helpers, no dependencies. Everything here is lifted 1:1
//! from the HTML prototype (`MartianHybrid.dc.html`) so the egui port matches.
//! The geometry is in the prototype's logical pixels at a 960×600 panel; keep
//! the ratios, exact px aren't sacred.

// ============================================================ LAYOUT
pub const PANEL_W: f32 = 960.0;
pub const PANEL_H: f32 = 600.0;
pub const TOPBAR_H: f32 = 46.0;     // full-width metal top bar
pub const GROOVE_H: f32 = 2.0;      // accent groove under the top bar
pub const MAIN_H: f32 = 552.0;      // body height (TOPBAR_H + GROOVE_H + MAIN_H = 600)

pub const LEFT_COL_W: f32 = 470.0;  // waterfall column; padding 8/10/8/14 (t/r/b/l)
pub const VGROOVE_W: f32 = 2.0;     // vertical groove between columns
// right column: flex (fills remainder ≈ 486 wide); padding 8/14/8/12

pub const GAP: f32 = 8.0;           // vertical gap between stacked panels
pub const HEADER_ROW_H: f32 = 24.0; // each panel's title row
pub const HEADER_GAP: f32 = 6.0;    // gap between title row and recessed screen

// Right-column panel heights (top→bottom). MAP is flex and fills the rest (~228).
pub const LOG_H: f32 = 142.0;
pub const BANDSCAN_H: f32 = 112.0;  // "half height" panel
pub const FOOTER_H: f32 = 30.0;
// Left column: header(24) + screen(flex) + ticker(30, gap 8). Ticker height is
// matched to FOOTER so the waterfall + contacts recessed screens bottom-align.
pub const TICKER_H: f32 = 30.0;

// Recessed-screen corner brackets: arm 9px, stroke 1.5px, accent, flush to corner.
// Panel title "spine" bar: 3px wide × 14px tall, accent.

// ============================================================ PALETTE
/// Solid colors only. Gradients (face, lcdBg, grooves, sheen) and the inset
/// "recessed screen" shadow are described in README.md — fake them in egui with
/// a vertex-colored Mesh + a 1px accent ring (see the egui spike handoff).
pub struct Palette {
    pub accent: u32,        // primary accent
    pub text: u32,          // body text
    pub legend: u32,        // headings / legends (brightest)
    pub sub: u32,           // sub-labels (also used for muted map dots / band numerals)
    pub dim: u32,           // dim data text
    pub screen_bg: u32,     // recessed screen fill
    pub edge: u32,          // chassis border
    pub lcd_text: u32,      // LCD readout text (clocks, dial)
    pub on_accent: u32,     // text drawn ON an accent fill (lit toggle keys, Scan)
}

pub const DARK: Palette = Palette {
    accent: 0xF7920F, text: 0xF4EEE6, legend: 0xF6E6CF, sub: 0xCAB496, // sub = rgba(202,180,150,.72)
    dim: 0xCDAF8C, screen_bg: 0x080604, edge: 0x100C08, lcd_text: 0xFFB24D, on_accent: 0x1D1408,
};
pub const LIGHT: Palette = Palette {
    accent: 0xC2660F, text: 0x241808, legend: 0x36260F, sub: 0x5F4420, // sub = rgba(95,68,32,.78)
    dim: 0x785028, screen_bg: 0xEFE7DC, edge: 0xA39880, lcd_text: 0x3A2A10, on_accent: 0xFDF6EC,
};
// Map land fill / coastline stroke (RGBA), per theme:
//   DARK : fill rgba(255,238,214,0.055)  stroke rgba(247,160,60,0.40)
//   LIGHT: fill rgba(95,62,20,0.10)      stroke rgba(150,80,10,0.45)

// Fonts: Chakra Petch (headings/legends/numerals, 600–700, tracked, UPPERCASE),
//        IBM Plex Mono (all data/body, 400–600). Both OFL — vendor the TTFs.

// ============================================================ MAP PROJECTION
// Equirectangular. SVG viewBox is 393×190; latitude window 16°..54°N.
pub const LON0: f32 = -150.0;   // left edge longitude
pub const LAT_TOP: f32 = 54.0;  // top edge latitude
pub const KX: f32 = 0.819;      // cos(midlat) longitude compression
pub const S: f32 = 5.0;         // units per degree of latitude
pub const MAP_W: f32 = 393.0;   // (= (−54 − LON0) * KX * S)
pub const MAP_H: f32 = 190.0;   // (= (LAT_TOP − 16) * S)

#[inline] pub fn map_x(lon: f32) -> f32 { (lon - LON0) * KX * S }
#[inline] pub fn map_y(lat: f32) -> f32 { (LAT_TOP - lat) * S }

pub const HOME_LAT: f32 = 40.00; // Lafayette, CO (QTH) — N0JDC, grid DN70KA
pub const HOME_LON: f32 = -105.10;

// Graticule
pub const MERIDIANS: &[f32] = &[-140.0,-130.0,-120.0,-110.0,-100.0,-90.0,-80.0,-70.0,-60.0];
pub const PARALLELS: &[f32] = &[20.0, 30.0, 40.0, 50.0];
// US/Canada border: dashed line at 49°N from left edge to lon −95 (lakes start).
pub const BORDER_LAT: f32 = 49.0;
pub const BORDER_LON_END: f32 = -95.0;
// Home range rings (great-circle approx as ellipses): for km d,
//   rx = (d / 85.0) * KX * S ,  ry = (d / 111.0) * S
pub const RING_KM: &[f32] = &[750.0, 1500.0];

// ============================================================ CONTACTS (fake)
#[derive(Clone, Copy)] pub enum Country { Us, Ca, Mx }
// Marker style: US = filled accent dot (r 2.4); CA = hollow ring (accent stroke,
// screen_bg fill, r 2.4); MX = filled `sub`-color dot (r 2.2).
pub struct Contact { pub call: &'static str, pub lat: f32, pub lon: f32, pub country: Country }
use Country::*;
pub const CONTACTS: &[Contact] = &[
    Contact{call:"K7RA", lat:47.6, lon:-122.3, country:Us}, Contact{call:"K7UT", lat:40.7, lon:-111.9, country:Us},
    Contact{call:"W7PH", lat:33.4, lon:-112.1, country:Us}, Contact{call:"K5ED", lat:32.8, lon:-96.8, country:Us},
    Contact{call:"N5JR", lat:29.8, lon:-95.4, country:Us},  Contact{call:"W9XYZ",lat:41.9, lon:-87.7, country:Us},
    Contact{call:"K1ABC",lat:42.4, lon:-71.1, country:Us},  Contact{call:"W4GH", lat:33.8, lon:-84.4, country:Us},
    Contact{call:"N4FL", lat:25.8, lon:-80.2, country:Us},  Contact{call:"VE7CC",lat:49.2, lon:-123.1,country:Ca},
    Contact{call:"VE6AO",lat:51.0, lon:-114.1,country:Ca},  Contact{call:"VE4XX",lat:49.9, lon:-97.1, country:Ca},
    Contact{call:"VE3EN",lat:43.7, lon:-79.4, country:Ca},  Contact{call:"XE2OK",lat:25.7, lon:-100.3,country:Mx},
    Contact{call:"XE2HW",lat:28.6, lon:-106.1,country:Mx},  Contact{call:"XE1RC",lat:19.4, lon:-99.1, country:Mx},
];

/// Great-circle distance (km) — used to label "Best DX".
pub fn haversine_km(la1: f32, lo1: f32, la2: f32, lo2: f32) -> f32 {
    let re = 6371.0_f32;
    let dl = (la2 - la1).to_radians();
    let dn = (lo2 - lo1).to_radians();
    let a = (dl * 0.5).sin().powi(2)
        + la1.to_radians().cos() * la2.to_radians().cos() * (dn * 0.5).sin().powi(2);
    2.0 * re * a.sqrt().min(1.0).asin()
}

// ============================================================ COASTLINE (fake/simplified)
// (lat, lon) waypoints, one closed polygon: West Coast → Baja + Gulf of California
// → mainland Mexico → Gulf of Mexico → Florida → East Coast → Atlantic Canada →
// straight top edge back. Project each point with map_x/map_y, fill with land
// color, stroke 0.6 with the coastline color. Low-detail on purpose (no Great
// Lakes / Hudson Bay).
pub const COAST: &[(f32, f32)] = &[
    (54.0,-132.0),(50.5,-128.0),(49.2,-123.6),(46.2,-124.0),(42.0,-124.3),(40.4,-124.4),(38.0,-123.0),
    (36.6,-121.9),(34.4,-120.5),(33.0,-117.4),(32.5,-117.2),(31.5,-116.6),(30.0,-115.8),(28.0,-114.5),
    (25.2,-112.0),(23.0,-110.0),(24.0,-110.3),(27.0,-111.3),(30.0,-113.0),(31.5,-114.5),(31.3,-113.3),
    (28.8,-111.6),(26.5,-109.3),(23.2,-106.4),(20.6,-105.4),(17.8,-101.8),(16.0,-98.6),(16.0,-94.6),
    (18.2,-94.5),(19.5,-96.2),(22.0,-97.5),(25.9,-97.2),(28.0,-96.5),(29.7,-93.8),(29.0,-90.0),
    (30.3,-88.9),(30.3,-86.5),(29.7,-84.0),(28.0,-82.8),(25.9,-81.7),(25.2,-80.4),(27.0,-80.1),
    (29.5,-81.0),(31.5,-81.0),(33.9,-78.2),(35.2,-75.5),(37.0,-76.0),(38.9,-74.9),(40.5,-74.0),
    (41.4,-71.0),(42.0,-70.2),(43.5,-70.3),(44.6,-67.5),(45.2,-66.0),(44.7,-63.5),(46.3,-60.3),
    (47.8,-59.0),(48.5,-54.5),(51.5,-55.5),(54.0,-57.0),
];

// ============================================================ WATERFALL DECODE RAIL (fake)
// Left panel: a sideways waterfall image with a decode rail down its right edge.
// Each decode sits at a vertical position derived from its audio frequency (Hz):
//   y_px = ((1 − f / 3000) * RAILH − 7).clamp(2, RAILH − 16)   with RAILH = 438
pub const FMAX_HZ: f32 = 3000.0;
pub const RAILH: f32 = 438.0;
pub const DECODES: &[(f32, &str, &str)] = &[ // (audio_hz, callsign, snr)
    (2680.0,"OH8X","−08"),(2510.0,"JA1NUT","−15"),(2360.0,"K1ABC","−02"),(2200.0,"DL3XYZ","−19"),
    (2050.0,"VK3WE","−21"),(1880.0,"W7GH","−11"),(1720.0,"EA7KW","−17"),(1560.0,"N5JR","−05"),
    (1400.0,"PY2OG","−23"),(1240.0,"G4ABC","−13"),(1080.0,"VE3EN","−09"),(920.0,"ZL2AB","−24"),
    (600.0,"UA9XYZ","−18"),
];

// ============================================================ LOG BOOK (fake, last 4)
pub const LOGS: &[(&str,&str,&str,&str,&str)] = &[ // (utc, call, grid, sent, rcvd)
    ("2358","W7GH","CN94","−11","−09"),("2355","JA1NUT","PM95","−15","−13"),
    ("2351","G4ABC","IO91","−13","−07"),("2347","VE3EN","FN25","−09","−02"),
];

// ============================================================ BAND SCAN (fake)
pub const BANDS: &[(&str,u32,u32)] = &[ // (band, heard, unworked)
    ("40m",23,7),("20m",41,12),("15m",18,9),("10m",6,4),
];
// Columns: left = [40m, 20m], right = [15m, 10m].
// Scan cycles activeBand 0→3 every 2.5 s, then returns to idle ("Last scan: just now").
