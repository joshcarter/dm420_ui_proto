# Handoff: Martian FT8 Console Panel → Rust + egui

## Overview

A single instrument-style panel for FT8 amateur-radio software, in the "Martian"
theme (brushed-metal chassis, recessed glass screens, amber accent, flat tactical
hardware). One panel, **960 × 600**, with a full-width top bar and a two-column
body. This supersedes the earlier `design_handoff_egui_spike/` (which proved out
the *chrome* only) — this bundle documents the **complete, evolved panel** so it
can be reimplemented accurately.

The panel contains six regions:
1. **Top bar** — station identity, live clocks, display + GUI mode switches.
2. **Waterfall** (left, large) — sideways spectrogram + decode rail.
3. **Log Book** (top-right) — recent QSOs.
4. **Band Scan** (mid-right, half-height) — multi-band scan status + counts.
5. **Contacts map** (right, fills remainder) — North America map with spots.
6. **Footer** — flat square-toggle status row + SNR bar-graph.

## About the design files

The files in `reference/` are **design references built in HTML**, not production
code to copy. The two `.dc.html` files render through a small in-house runtime
(`support.js`); egui has no DOM/CSS, so **reason from the documented values, the
geometry, and the data tables — not the markup**. The task is to re-create this
panel in **Rust + egui** (the project already has an egui spike), reusing that
codebase's patterns. `reference/panel_data.rs` is the one file meant to be used
directly: it's pure data (palette, projection, coastline, all fake-data tables).

- `reference/MartianHybrid.dc.html` — the panel renderer (source of truth for
  layout, geometry, and the per-render data in its `<script>`).
- `reference/Martian Hybrid.dc.html` — board that mounts the panel in dark + light;
  its `<script>` holds the **full dark/light token objects** (gradients, shadows).
- `reference/panel_data.rs` — palette + projection + coastline + fake data, ready to drop in.
- `reference/wf-martian-dark.png`, `reference/wf-martian-light.png` — the waterfall
  spectrogram images used by the left panel (use as-is, or generate live FFT later).

## Fidelity

**High-fidelity on chrome and layout** (exact colors, geometry, typography below);
**throwaway on data** — every number/callsign/spot is fake placeholder content
(see `panel_data.rs`). Match the look precisely; the data is illustrative and will
later be replaced by live radio state (see "Swapping in real data").

## Layout

```
┌───────────────────────────────── 960 ─────────────────────────────────┐
│ TOP BAR (h 46)  identity · clocks · DISPLAY/GUI switches                │
├──────────────────────────── groove (h 2) ─────────────────────────────┤
│ MAIN (h 552)                                                           │
│ ┌── WATERFALL ───────┐│┌── RIGHT COLUMN (flex) ──────────────────────┐ │
│ │ left col, w 470     ││ Log Book        h 142                        │ │
│ │  header 24          ││  (gap 8)                                     │ │
│ │  recessed screen    ││ Band Scan       h 112  (half height)        │ │
│ │   (flex)            ││  (gap 8)                                     │ │
│ │  ticker  h 30       ││ Contacts map    flex (≈228)                 │ │
│ │                     ││  (gap 8)                                     │ │
│ │                     ││ Footer          h 30                         │ │
│ └─────────────────────┘└──────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────────────┘
```

- Top bar + groove + main = 46 + 2 + 552 = **600**.
- Left column width **470** (padding t/r/b/l = 8/10/8/14); then a **2px** vertical
  groove; then the right column (flex, padding 8/14/8/12).
- Right column children stack with **8px** gaps: Log 142, Band Scan 112, Map (flex),
  Footer 30. The Map flexes to fill (~228).
- **Bottom-align rule:** the left **ticker (30)** matches the right **footer (30)** so
  the waterfall and contacts recessed screens end on the same line. Keep this if you
  change heights.
- Every panel = a **24px title row** (3px×14px accent spine bar + tracked UPPERCASE
  legend + dim sub-label) then a **6px** gap then a **recessed screen** with flush
  accent **corner brackets** (arm 9px, stroke 1.5px).

All geometry constants are in `panel_data.rs` (LAYOUT section).

## Components

### Top bar (h 46, padding 0/24/0/14)
- **Identity** (left): 3px×16px accent spine bar; then `N0JDC` (Chakra Petch 700,
  18px, +0.12em, `legend`) + `DN70KA` (9px, +0.18em, UPPERCASE, `sub`).
- **Clocks** (right, gap 10): two recessed LCD chips (bg `lcdBg` gradient, inset
  shadow, radius 3, padding 3/12). Each: small label `LOCAL`/`UTC` (8px, `lcdText`
  @0.6) + readout (Chakra Petch 700, 16px, `lcdText`, glow). **The readout has a
  fixed width (79px, centered)** so it doesn't reflow as digits tick — replicate
  with a fixed cell or tabular figures.
- **Switches** (gap 14): two segmented controls, each a recessed track (bg `lcdBg`,
  inset shadow, radius 4, 2px pad, 2px gap) of key cells (padding 5/11, radius 2,
  Chakra Petch, 9px, +0.1em). **Active cell = solid `accent` fill, text `on_accent`,
  raised shadow** (`inset 0 1px 0 rgba(255,255,255,.28), 0 1px 2px rgba(0,0,0,.45)`);
  inactive = transparent, text `sub`, weight 600. Each has a 7px UPPERCASE micro-label
  above (`DISPLAY`, `GUI`).
  - **DISPLAY**: `DARK` / `LIGHT` — reflects the active theme.
  - **GUI**: `LOCK` / `EDIT` — interactive, default LOCK.

### Waterfall (left, w 470)
- Title row: `Waterfall` + sub `0–3000 Hz · time → left` + right-aligned `● SPLIT  AGC`.
- Recessed screen is a 3-column grid: **28px** freq axis (`3k/2k/1k/Hz` ticks, 8px
  `dim`) | **1fr** spectrogram (`wf-martian-*.png`, object-fit fill; a 2px accent
  "NOW" line on the right edge + `NOW` label; `−60s` bottom-left) | **124px** decode
  rail. Rail items are absolutely positioned at `y = topPx` (from audio freq; see
  `DECODES` + formula in `panel_data.rs`): a 7px accent tick + 5px LED dot + callsign
  (Chakra Petch 600, 10px) + right SNR (8.5px `dim`).
- Ticker row (h 30): scrolling-style mono line, e.g. `235945 −02 K1ABC W9XYZ EM48 · …`.

### Log Book (top-right, h 142)
- Title: `Log Book` + sub `last 4 · FT8` + right `312 QSO`.
- Recessed screen: a grid header (`UTC / Call / Grid / Snt / Rcv`, 8px `dim`,
  columns `50px 1fr 60px 48px 48px`) then 4 rows (h 22, 10px). `Rcv` is `accent`.
  Data: `LOGS` in `panel_data.rs`.

### Band Scan (mid-right, h 112 — half height)
- Title: `Band Scan` + status sub + a **Scan/Cancel button** styled exactly like the
  top toggle keys (recessed track + lit accent key, `on_accent` text).
- Recessed screen: **two columns** split by a 1px divider. Left = 40m, 20m; right =
  15m, 10m. Each band block: large numeral (Chakra Petch 700, 22px, color = `sub`
  when idle / `accent` when that band is actively scanning) + two small lines
  (11px): `<heard> heard` (count in `legend`) and `<unworked> unworked` (count in
  `accent`). Active band also gets a 2px accent left-border. Data: `BANDS`.

### Contacts map (right, flex ≈228)
- Title: `Contacts` + sub `N. America · DN70KA` + right `16 spots`.
- Recessed screen holds an SVG-style map at **viewBox 393×190** (`xMidYMid meet`),
  drawn back-to-front:
  1. **Land polygon** (`COAST` projected) — subtle land fill + 0.6 coastline stroke.
  2. **Graticule** — meridians (`MERIDIANS`) + parallels (`PARALLELS`, labeled
     `20°…50°`), 0.4 `dim` @0.25.
  3. **49°N border** — dashed accent line, left edge → lon −95.
  4. **Range rings** — dashed accent ellipses around home (`RING_KM`: 750/1500 km;
     rx/ry formulas in `panel_data.rs`).
  5. **Region labels** `CANADA`, `MÉXICO` (Chakra Petch, `sub`).
  6. **Contact spots** (`CONTACTS`) — US filled accent dot, CA hollow ring, MX muted
     dot; callsign label beside each (4.8px mono; anchor flips near the right edge).
  7. **QTH marker** — accent ring (r 4.6) + crosshair + center dot + `QTH` label.
- Projection + all coordinates: `panel_data.rs` (MAP section). Home = Lafayette, CO.

### Footer (h 30)
- Flat square toggles (10px): solid accent = on (`DX ONLY`, `LOG`), 1.5px hollow =
  off (`CQ`, `ALERT`); 8.5px tracked labels. Right: a 6-bar accent/`sub` bar-graph
  + `SNR`. **No screws, no glossy switches** (removed from this theme).

## Interactions & behavior

- **Clocks**: `LOCAL` = browser-local 24h `HH:MM:SS`; `UTC` = UTC `HH:MM:SS`; both
  tick once per second. In egui, format `chrono::Local::now()` / `Utc::now()` each
  frame (or on a 1s timer) and `ctx.request_repaint_after(1s)`.
- **DISPLAY switch**: reflects/sets the active palette (dark/light). On the board it
  only *reflects* (the board shows both side by side); in a single live app, wire it
  to toggle the theme — swap the `Palette` and regenerate the brushed texture.
- **GUI switch (LOCK/EDIT)**: local boolean, default LOCK; clicking sets it. (Intended
  to gate a future drag-to-rearrange edit mode.)
- **Band Scan**: clicking **Scan** starts a cycle — `activeBand` advances 0→3, one band
  every **2.5 s**, highlighting that band (accent numeral + left bar) and showing
  `Scanning {band} …`; after the last band it stops and shows `Last scan: just now`.
  The button reads **Cancel** while running and stops immediately. (Real behavior:
  each step blocks TX and decodes one FT4/FT8 interval on that band.)

## State management

Minimal local UI state:
- `theme: Dark | Light`
- `gui_mode: Lock | Edit`
- `scan: { running: bool, active_band: usize, last_scan_min: u32 }` + a timer/accumulator
- clock values (derived each frame from the system clock)

Everything else (decodes, log, bands, contacts, coastline) is static fake data today.

## Design tokens

Fonts: **Chakra Petch** (headings/legends/numerals, 600–700, tracked, UPPERCASE) and
**IBM Plex Mono** (data/body, 400–600 → `FontFamily::Monospace`). Both OFL; vendor TTFs.

Solid colors (also in `panel_data.rs`):

| Role | DARK | LIGHT |
|---|---|---|
| accent | `#F7920F` | `#C2660F` |
| text | `#F4EEE6` | `#241808` |
| legend (headings) | `#F6E6CF` | `#36260F` |
| sub | `rgba(202,180,150,.72)` | `rgba(95,68,32,.78)` |
| dim | `rgba(205,175,140,.60)` | `rgba(120,80,40,.62)` |
| screen bg | `#080604` | `#EFE7DC` |
| chassis edge | `#100C08` | `#A39880` |
| LCD text | `#FFB24D` | `#3A2A10` |
| on-accent (text on a lit key) | `#1D1408` | `#FDF6EC` |
| map land fill | `rgba(255,238,214,.055)` | `rgba(95,62,20,.10)` |
| map coastline | `rgba(247,160,60,.40)` | `rgba(150,80,10,.45)` |

Gradients & effects (exact strings in `reference/Martian Hybrid.dc.html` `<script>`):
- **face** (chassis): `linear-gradient(180deg,#2C2823→#171310)` dark / `#E9E4D9→#CDC6B7` light.
- **brushed overlay**: 1px vertical stripes — dark `rgba(255,255,255,.022)` over
  `rgba(0,0,0,.03)`; light `rgba(255,255,255,.55)` over `rgba(0,0,0,.035)`.
- **lcdBg**: `linear-gradient(180deg,#1C1407→#0D0A04)` dark / `#E8DCC0→#D4C69E` light.
- **recessed screen inset**: `inset 0 2px 9px rgba(0,0,0,.9), inset 0 0 0 1px rgba(247,146,15,.4)`
  (light: `inset 0 2px 7px rgba(0,0,0,.22), inset 0 0 0 1px rgba(150,80,10,.42)`).
- **chassis bevel**, **grooves**, **sheen**, **LED gradients/glow** — see the same script.

Radii: chassis 4px; recessed screens 0px (squared). Corner brackets: arm 9, stroke 1.5,
flush. Spine bar: 3×14. Footer toggles: 10px square. Spacing: panel gaps 8, header
row 24 + 6 gap.

## egui technique map

| Element | egui approach | Difficulty |
|---|---|---|
| Panel frames / strokes / fills / 1px accent ring | `egui::Frame` | Easy |
| Corner brackets, spine bars, footer bar-graph, square toggles | `Painter::line_segment` / `rect_filled` / `rect_stroke` | Easy |
| Custom fonts | `FontDefinitions` + vendored TTFs | Easy |
| Light/dark theme swap | `Palette` struct + `ctx.set_visuals` + regen texture | Easy |
| Segmented switches / Scan button (lit key) | two `Frame` keys; active = accent fill + `on_accent` text + raised shadow | Easy |
| Live clocks (fixed-width) | `chrono` + monospace/tabular, `request_repaint_after` | Easy |
| Face / LCD **gradients** | vertex-colored `Mesh` (top→bottom) | Medium |
| **Brushed-metal texture** | generate a 1px-stripe `ColorImage` once → `TextureHandle`, tile behind panes | Medium |
| **Recessed/inset bevel** | no inset-shadow primitive: 1px accent ring + short top-edge gradient Mesh | Medium |
| Waterfall image | load PNG as texture, draw into the spectro cell (or live FFT later) | Easy |
| **Contacts map** | project `CONTACTS`/`COAST` with `map_x/map_y`; `Painter` polygon (land), lines (graticule), ellipses (rings), circles (spots), `text` (labels) | Medium |
| Band-scan timer/highlight | accumulate `stable_dt`; advance every 2.5 s | Easy |

The genuinely uncertain bits worth care: **egui_tiles chrome suppression**, the
**brushed texture + grooves-through-gaps**, and **recessed-panel emulation** — all
already evaluated in `design_handoff_egui_spike/FEASIBILITY.md`; follow that.

## Swapping in real data later

The fake tables are isolated so the UI never changes when real data arrives:
- **Waterfall**: replace the static PNG with live FFT magnitudes → a scrolling
  texture (the `design_handoff_waterslide/` bundle has a full Rust pattern for this).
- **Decodes / Log / Band counts / Contacts**: replace the `const` tables in
  `panel_data.rs` with values from the decoder / logbook / scanner. The map already
  plots by lat/lon, so contact spots come straight from grid-square → lat/lon.
- **Future**: the doc notes band-scan results should also drop pips on the map — same
  `CONTACTS`-style list, projected the same way.

## Acceptance criteria

- [ ] `cargo run` shows the 960×600 panel; brushed chassis, recessed screens, flush
      accent brackets, spine-marked headers.
- [ ] Top bar: identity, two **fixed-width** ticking clocks (LOCAL 24h + UTC), and the
      DISPLAY + GUI segmented switches (lit accent key = active).
- [ ] Waterfall image + decode rail; Log Book (4 rows); Footer toggles + SNR bars.
- [ ] Band Scan: two-column band blocks with heard/unworked counts; **Scan** cycles
      the bands (2.5 s each) with the active band highlighted, then resets.
- [ ] Contacts map: North-America coastline (land vs ocean), graticule, 49°N border,
      home QTH + range rings, and the 16 spots placed by lat/lon with country styling.
- [ ] Both palettes render correctly and switch at runtime.

## Files

- `README.md` — this brief.
- `reference/panel_data.rs` — **drop-in** palette + projection + coastline + fake data.
- `reference/MartianHybrid.dc.html` — panel renderer (geometry + per-render data).
- `reference/Martian Hybrid.dc.html` — board with the full dark/light token objects.
- `reference/support.js` — the prototype runtime (context only; do not port).
- `reference/wf-martian-dark.png` / `-light.png` — waterfall spectrogram images.

See also the sibling bundles in this project: `design_handoff_egui_spike/`
(chrome feasibility + verdict) and `design_handoff_waterslide/` (live scrolling-FFT
Rust pattern).
