# Handoff: "Waterslide" panel → egui (with fake transmissions & decodes)

## Goal

Get the **Waterslide** FFT panel from the HTML prototype running inside your
existing egui spike — **not** a functioning radio yet, but a live animated mock
fed by **fake transmissions and decodes**, exactly like the web version.

A "waterslide" is a waterfall rotated 90°: **time runs horizontally, frequency
vertically.** The panel is split down the middle at **"now"**:

```
   DECODED TEXT  (left half)         |        FFT WATERSLIDE  (right half)
   oldest …→… newest                 | newest …→… oldest
   scrolls LEFT                    [ NOW ]               scrolls RIGHT
                                                        ┌─ ±Hz scale (far right)
```

Both halves show the same **30 seconds**. A signal at audio offset *f* appears as
a horizontal streak at vertical position *f* in the FFT; its decoded text sits at
the **same vertical position** on the left, so you can read multiple senders
across at a glance. See `reference/waterslide-dark.png` and, for exact geometry,
`reference/Waterslide.dc.html`.

## The hard part is already done — `waterslide_sim.rs`

**You do not need to invent the fake data.** `reference/waterslide_sim.rs` is a
dependency-free, 1:1 port of the prototype's generator. Everything is a pure
function of the simulated clock `t` (seconds), so it's deterministic and looks
identical to the mockup. It gives you:

- `POOL` — 14 fake stations (callsign, grid, audio offset, strength, TX odds).
- `column_values(ts, col_id, &mut [f32; 500])` — one vertical FFT column of
  intensities for an instant `ts` (noise floor + 8-FSK signal streaks). Map each
  value through a colormap to get pixels.
- `Sim` — advance the clock with `advance(dt)`, then `records()` returns the
  decoded lines currently on screen, each with `td` (decode time → horizontal
  scroll) and `final_y` (frequency lane, already de-collided).

That's the whole simulation. Copy it in unchanged.

## The rendering is three egui techniques — `waterslide_panel.rs`

`reference/waterslide_panel.rs` is a working reference `WaterslidePanel` you can
drop into a pane. The three moving parts:

1. **Scrolling spectrogram = a texture you shift in place.**
   Keep a `ColorImage` (520×500). Each frame, shift every row right by the number
   of pixels time advanced (`copy_within`, memmove-safe), then write the new
   columns at the left edge from `column_values(...)`, and re-upload with
   `TextureHandle::set`. Paint it with `painter.image(...)`. This is the egui
   analogue of the canvas "draw-self-offset + putImageData" trick, and it keeps
   frozen history stable while only the leading edge updates.
   *(Simpler alt: rebuild the whole image every frame from the model — 260 k px
   is cheap in native Rust. The noise floor is keyed on `col_id`, so history
   stays stable either way. The scroll-blit version just does less work.)*

2. **Decoded text = `painter.text` at computed scroll positions.**
   For each `DecodeRec`, `x = 520 − (t − td)·PPS` (PPS = 520/30 px·s⁻¹); draw the
   message right-aligned (`Align2::RIGHT_CENTER`) at `(x, final_y)`, the SNR just
   left of it. Wrap the lane in `ui.painter_at(text_rect)` so lines clip cleanly
   as they slide off. No DOM, no per-line widgets — just painted glyphs.

3. **NOW line, ±Hz scale, graticule = `Painter` primitives** (`line_segment`,
   `text`) — same family of calls you already use for the corner brackets.

Animation: the panel calls `ctx.request_repaint()` every frame; drive the clock
from `ui.input(|i| i.stable_dt)`.

## Drop-in steps (into your egui_tiles spike)

1. Copy `waterslide_sim.rs` and `waterslide_panel.rs` into `src/` and `mod` them.
2. Hold one panel in app state: `waterslide: WaterslidePanel::new(7200.0)` (any
   start time in seconds; pass a persisted value if you want continuity).
3. In the **"Waterfall" pane's `pane_ui`**, after you've painted the recessed
   screen + brackets, hand the inner rect to the panel:
   ```rust
   let dt = ui.input(|i| i.stable_dt) as f64;
   self.waterslide.ui(ui, dt, &theme.waterslide);
   ```
4. Build a `WaterslideTheme` per palette and swap it on your existing light/dark
   (or variant) toggle — it carries the `accent/text/dim/screen_bg/grid` colours
   plus the **colormap**. The colormap is the only new token vs. your spike.

This reuses the chassis, recessed-screen, bracket and font work from the spike
verbatim; the Waterslide is just what goes *inside* one pane.

## egui technique map (the Waterslide-specific bits)

| Element | egui approach | Difficulty |
|---|---|---|
| Scrolling FFT image | `ColorImage` + `TextureHandle::set` each frame; `Painter::image` | **Medium** — the one genuinely new technique |
| Per-pixel colormap | precomputed `[Color32; 256]` LUT | Easy |
| Decoded text scroll | `Painter::text` + `painter_at` clip | Easy |
| NOW line / graticule / scale | `line_segment`, `text` | Easy |
| Frame-driven animation | `ctx.request_repaint()` + `stable_dt` | Easy |
| Fake data | `waterslide_sim.rs` (provided) | **Done** |

The only thing worth probing is texture-upload cost: re-uploading a 520×500
RGBA image every frame is ~1 MB/frame and totally fine on desktop, but confirm it
on your target. If you ever want to avoid the re-upload, the scroll could move to
a GPU shader — out of scope for the mock.

## Geometry / tokens

- FFT half **520×500** px (model space); **30 s** across → `PPS = 520/30 ≈ 17.33`.
- Vertical span **3000 Hz** (±1500 around the dial); `y_of(off) = 250 − off/3000·500`.
- Band per signal ≈ 20 px tall; 8 FSK tones at 1.7 px spacing (already in the sim).
- Layout bands (design x): text `0..520` · NOW `520..522` · FFT `522..1042` ·
  scale `1042..1118`. Total inner width **1118**.
- Decode line: 12 px mono message, 10.5 px SNR, lane gap 16 px (de-collision).
- Colormaps (intensity 0→1, the only new palette tokens):
  - **Martian** (amber): `(0,8/6/4) (.22,46/18/6) (.42,110/44/8) (.60,186/86/12) (.78,240/150/40) (1,255/236/205)`
  - **Phosphor** (green): `(0,4/9/6) (.25,8/40/20) (.45,14/92/46) (.64,36/158/82) (.82,110/224/140) (1,214/255/224)`
  - **Spectral** (cyan): `(0,4/9/12) (.25,8/38/56) (.45,14/84/118) (.64,34/142/190) (.82,110/200/236) (1,222/246/255)`

  Accent / text / dim for each palette match the Martian-Hybrid token sheet in the
  earlier handoff; the Martian accent is `#F7920F` (dark) / `#C2660F` (light).

## What "fake" means here — and swapping in real data later

The mock and a real receiver differ in only **two** functions; the rendering and
layout never change:

- **Real spectrum:** replace the signal-synthesis loop inside `column_values`
  with your live FFT magnitude bins (map dB → 0..1). Keep the noise floor or drop
  it. Everything downstream (texture, scroll, scale) is unchanged.
- **Real decodes:** stop calling `Sim::decodes_for_slot`. Instead, when your FT8
  decoder emits a line, push a `DecodeRec { td: now, final_y: y_of(audio_hz),
  msg, rsnr }`. The de-collision pass is one short function you can reuse.

So this spike doubles as the integration skeleton: wire the panel now, replace
two data sources later.

## Acceptance criteria (for the mock)

- [ ] The "Waterfall" pane shows a **scrolling** waterslide: signal streaks drift
      right, dead air between 15 s slots, noise floor shimmering at the leading edge.
- [ ] Decoded text scrolls **left** on the other half; each line sits at its
      signal's **frequency height**; lines don't overlap (de-collision works).
- [ ] **NOW** line down the centre; compact **±Hz scale** on the right reading
      `14.074` at centre, `+1500…−1500` above/below.
- [ ] Animation is smooth and continues without interaction.
- [ ] The palette (incl. colormap) switches with the existing light/dark toggle.

## Files

- `README.md` — this brief.
- `reference/waterslide_sim.rs` — **the fake-data engine** (drop in unchanged).
- `reference/waterslide_panel.rs` — **egui rendering reference** (adapt to your version).
- `reference/Waterslide.dc.html` — the prototype (source of truth for geometry/feel).
- `reference/waterslide-dark.png` — screenshot of the intended look (Martian/dark).
