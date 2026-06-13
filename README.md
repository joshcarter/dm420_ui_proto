# Handoff: "Martian Hybrid" theme — egui feasibility spike

## Goal

This is **not** a request to reproduce a UI pixel-perfectly. It is a **feasibility spike**.
The question we want answered, with evidence:

> Can the "Martian Hybrid" instrument-panel theme (brushed-metal chassis, distinct
> recessed panels, flush accent corner-brackets, spine-marked headers, flat toggles,
> hard contrast — in both light and dark) be implemented in Rust + **egui** without a
> huge amount of headache, especially when panels are laid out with **egui_tiles**?

Deliver a **small runnable app** that proves out the *chrome* (the theme), plus a written
verdict. **Panel contents do not matter** — fill them with throwaway labels, a couple of
fake table rows, a circle, whatever. What matters is that several panels render at once
with the correct framing/texture/brackets and that the whole thing flips between a light
and a dark palette at runtime.

## About the reference files

The files in `reference/` are **design references built in HTML** — a prototype showing the
intended look, not code to port. egui has no CSS or DOM, so reason from the *visual target*
(the screenshots and the token sheet below), not from the markup. The `.dc.html` files are
included only so you can inspect exact values and geometry if a screenshot is ambiguous.

- `reference/dark-top.png`, `reference/dark-bottom.png` — dark "graphite" palette
- `reference/light-top.png`, `reference/light-bottom.png` — light "silver" palette
- `reference/MartianHybrid.dc.html` — the single-panel renderer (source of truth for geometry)
- `reference/Martian Hybrid.dc.html` — the board that shows both palettes side by side

Fidelity: **hi-fi** on color/geometry of the *chrome*, **throwaway** on contents.

---

## What to build

A `cargo` binary crate using **eframe / egui** with **egui_tiles** for the layout.

Suggested dependencies (use current compatible versions; pin whatever `cargo add` resolves):

```toml
[dependencies]
eframe = "0.x"        # pulls egui + a backend
egui = "0.x"
egui_tiles = "0.x"
egui_extras = "0.x"   # optional, only if you want a quick TableBuilder in one pane
```

### Layout — use egui_tiles with *splits*, not tabs

We want **several panels visible simultaneously** (a dashboard), so build the tree from
linear/grid containers (horizontal + vertical splits), **not** a tab container. Tabs would
hide panels behind each other and defeat the point. A reasonable starting tree:

```
Vertical
├── (top header strip — can be a normal egui TopBottomPanel above the tree, see below)
└── Horizontal
    ├── Pane: "Waterfall"   (large, left)
    └── Vertical
        ├── Pane: "Log"
        └── Pane: "Contacts"  (with the flat-toggle footer)
```

Put the **header strip** (title + light/dark toggle + a fake frequency readout) in an
`egui::TopBottomPanel::top` so it spans full width, and run the `egui_tiles::Tree` inside
the `CentralPanel`. That mirrors the reference (full-width metal top bar, paneled body).

`egui_tiles` essentials:
- Define `struct Pane { title: String, kind: PaneKind }`.
- Implement `egui_tiles::Behavior<Pane>`:
  - `pane_ui(...)` — **this is where the theme work happens** (see below).
  - `tab_title_for_pane(...)` — return the title.
  - Override the visual hooks to flatten the library's default chrome: set a small
    `gap_width(...)` (so the brushed chassis shows *between* panes as the "grooves"),
    and recolor/flatten tab styling. Exact method names vary by version — check the
    `Behavior` trait docs for the version you pin (`gap_width`, `simplification_options`,
    `tab_bar_height`, and the `tab_*`/container color hooks).
- Build the tree with `egui_tiles::Tiles` + container helpers, then `tree.ui(&mut behavior, ui)`.

> ⚠ **This is the main thing to evaluate.** egui_tiles draws its own container/tab chrome and
> owns the resize handles and drag-reorder visuals. Getting our flat tactical look means
> suppressing/recoloring that chrome and painting our own frames + brackets inside `pane_ui`.
> Report how cleanly that worked.

### Per-panel chrome (inside `pane_ui`)

For each pane:
1. Get the pane's rect (`ui.available_rect_before_wrap()` / the rect egui_tiles gives you).
2. Paint the **recessed screen**: fill `screen_bg`, then stroke a **1px accent ring** around
   the rect. Emulate the inset/recessed feel with a short top-edge vertical gradient
   (dark → transparent) — see "gradients" below.
3. Draw the **corner brackets**: four L-shapes, **flush to the pane corners** (arm length ≈ 9px,
   stroke ≈ 1.5px, accent color), via `ui.painter().line_segment(...)`. In the latest design
   these sit *at* the corner (0px inset), not floating inside.
4. Draw a **header row**: a 3px-wide × ~14px-tall accent "spine" bar, then an uppercase,
   letter-tracked label in the heading font + a dim sub-label.
5. Put any throwaway content below.
6. In **one** pane, add the **footer**: a row of flat 10px **square** toggles — *solid accent
   fill = on*, *1.5px hollow stroke = off* — plus a little flat bar-graph. **No glossy slide
   switches, no screws** (those were deliberately removed from this theme).

### Chassis background (behind the panes)

Fill the central area with the **brushed-metal chassis**: a vertical face gradient overlaid
with faint 1px vertical stripes. The `gap_width` between panes lets this show through as the
grooves separating panels.

### Fonts

Bundle and load two fonts via `FontDefinitions` (download the `.ttf` from Google Fonts — both
are OFL-licensed, fine to vendor into `assets/`):
- **Chakra Petch** — headings, legends, the frequency numerals. Uppercase, tracked, weight 600–700.
- **IBM Plex Mono** — all data/body text, weight 400–600. Map to `FontFamily::Monospace`.

### Light / dark toggle (required)

A `Theme` struct holding the active palette. A toggle in the header switches it. On switch:
- `ctx.set_visuals(...)` for egui's own widgets, **and**
- swap the palette your custom frames/brackets/texture read from, **and**
- swap (or recolor/regenerate) the brushed-metal texture — the stripe colors differ light vs dark.

---

## Design tokens

Geometry (logical px at the reference's 960×600 panel scale — keep ratios, exact px not critical):
- Chassis corner radius **4px**; recessed screens **squared (0px)**.
- Corner brackets: arm **9px**, stroke **1.5px**, **flush to corner**.
- Header spine bar: **3px** wide × **14px** tall, accent fill.
- Footer toggle squares: **10px**; solid = on, 1.5px hollow stroke = off.
- Header strip ≈ 46px tall; per-pane header row ≈ 24px; footer row ≈ 30px.
- Tracked uppercase labels: letter-spacing ≈ 0.16–0.18em (see "known annoyances").

### DARK — "graphite"
| Role | Value |
|---|---|
| Chassis face (gradient top→bottom) | `#2C2823` → `#171310` |
| Chassis edge / border | `#100C08` |
| Brushed stripes | `rgba(255,255,255,0.022)` over `rgba(0,0,0,0.03)`, 1px period |
| Legend / heading text | `#F6E6CF` |
| Sub / dim label | `rgba(202,180,150,0.72)` |
| **Accent** | `#F7920F` |
| Recessed screen bg | `#080604` |
| Screen accent ring (1px inset) | `rgba(247,146,15,0.40)` |
| Body text | `#F4EEE6` |
| Dim data text | `rgba(205,175,140,0.60)` |
| LCD window bg (gradient) | `#1C1407` → `#0D0A04` |
| LCD text | `#FFB24D` |

### LIGHT — "silver"
| Role | Value |
|---|---|
| Chassis face (gradient top→bottom) | `#E9E4D9` → `#CDC6B7` |
| Chassis edge / border | `#A39880` |
| Brushed stripes | `rgba(255,255,255,0.55)` over `rgba(0,0,0,0.035)`, 1px period |
| Legend / heading text | `#36260F` |
| Sub / dim label | `rgba(95,68,32,0.78)` |
| **Accent** | `#C2660F` |
| Recessed screen bg | `#EFE7DC` |
| Screen accent ring (1px inset) | `rgba(150,80,10,0.42)` |
| Body text | `#241808` |
| Dim data text | `rgba(120,80,40,0.62)` |
| LCD window bg (gradient) | `#E8DCC0` → `#D4C69E` |
| LCD text | `#3A2A10` |

---

## egui technique map (what's cheap vs. what needs care)

Use this to structure the evaluation — confirm or correct each row in your write-up.

| Theme element | egui approach | Expected difficulty |
|---|---|---|
| Panel frames, strokes, radius, fills | `egui::Frame` (fill / stroke / rounding / inner_margin) | Easy |
| Corner brackets, spine bars, bar-graph | `Painter::line_segment` / `rect_filled` | Easy |
| Flat square toggles | `rect_filled` (on) / `rect_stroke` (off) | Easy |
| Custom fonts | `FontDefinitions` + vendored TTFs | Easy |
| egui's own widget theming | `Visuals` overrides + `ctx.set_visuals` | Easy |
| Face & LCD **gradients** | `egui::Mesh` with per-vertex colors (top/bottom) — egui supports vertex-colored meshes, so true gradients are doable | Medium |
| **Brushed-metal texture** | generate a small `ColorImage` of 1px stripes once, upload as a `TextureHandle`, tile/paint behind panes (or a `Mesh`) | Medium |
| **Recessed / inset bevel** | no inset-shadow primitive: fake with the 1px accent ring + a short top-edge gradient `Mesh` | Medium |
| Engraved text-shadow on legends | draw the label twice with a 1px offset | Easy-ish |
| `letter-spacing` on tracked caps | egui `RichText` has no letter-spacing — accept default, pad with thin spaces, or hand-lay glyphs | Minor annoyance |
| Glows (LCD text) | layered translucent shapes — *mostly avoidable*, this theme is largely flat | Medium, optional |

The genuinely uncertain parts worth the spike are **egui_tiles chrome suppression**, the
**brushed texture + grooves-through-gaps**, and the **recessed-panel emulation**. Everything
else is routine egui.

---

## Acceptance criteria (what the spike must demonstrate)

- [ ] `cargo run` launches the app.
- [ ] A full-width header strip with the title, a fake frequency readout, and a **light/dark toggle**.
- [ ] **≥3 panels visible at once**, laid out and resizable via **egui_tiles** (splits, not tabs).
- [ ] The brushed-metal chassis shows behind/between panels (gaps read as grooves).
- [ ] Each panel: recessed screen fill + **1px accent ring** + **accent corner brackets flush to the corners** + a **spine-marked header**.
- [ ] One panel carries the **flat square-toggle footer** (solid = on, hollow = off). No screws, no glossy switches.
- [ ] Correct fonts loaded (Chakra Petch headings, IBM Plex Mono data).
- [ ] **Both palettes switch at runtime** and look right (chassis, screens, brackets, text, texture all update).

## Deliverable: `FEASIBILITY.md`

Alongside the running crate, write a short `FEASIBILITY.md` with:
1. **Verdict** — easy / moderate / painful, one line.
2. **Per-technique notes** — confirm/correct the technique-map table; what looked great, what fought egui or egui_tiles.
3. **egui_tiles assessment** — how hard was it to suppress the default chrome and paint our own frames/brackets? Did resize/drag visuals cooperate?
4. **Rough effort estimate** to take this from spike → polished production theme.
5. **Recommendation** — go / adjust-the-design / avoid, with reasons.

Include 1–2 screenshots of your running spike (light and dark) in the repo.

## Files in this bundle
- `README.md` — this brief.
- `reference/*.png` — the four reference screenshots (light/dark, top/bottom).
- `reference/MartianHybrid.dc.html` — single-panel renderer (exact geometry/values).
- `reference/Martian Hybrid.dc.html` — both-palette board.
