//! Waterslide — egui rendering reference.
//!
//! A self-contained panel that paints the Waterslide visualisation from
//! `waterslide_sim`. Written against eframe/egui ~0.27–0.29; a couple of API
//! names (`load_texture`, `TextureHandle::set`, `painter_at`) have been stable
//! for many versions but double-check against the version you pin.
//!
//! Drop `WaterslidePanel` into the "Waterfall" pane of your egui_tiles spike:
//!     // once, in your app state:
//!     waterslide: WaterslidePanel::new(7200.0),   // any start time in seconds
//!     // inside `pane_ui` for that pane:
//!     let dt = ui.input(|i| i.stable_dt) as f64;
//!     self.waterslide.ui(ui, dt, &theme.waterslide);   // theme = your active palette
//!
//! The panel calls `ctx.request_repaint()` so it animates continuously.

use eframe::egui::{
    self, Align2, Color32, ColorImage, FontId, Pos2, Rect, Stroke, TextureHandle, TextureOptions,
};

use crate::waterslide_sim::{column_values, Sim, COL_H, SLOT, TX};

// FFT half is W×H px in model space; 30 s spans W px → PPS px/sec.
const W: usize = 520;
const H: usize = COL_H; // 500
const PPS: f64 = W as f64 / 30.0;

// Design-space x bands inside the screen (text | divider | fft | scale).
const X_TEXT_R: f32 = 520.0; // right edge of the text lane == "now" centre
const X_FFT_L: f32 = 522.0;
const X_FFT_R: f32 = 1042.0;
const X_SCALE_L: f32 = 1042.0;
const DESIGN_W: f32 = 1118.0;

/// Palette the panel reads. Build one per theme (martian/phosphor/spectral) and
/// swap it on your light/dark (or variant) toggle — same hook as the spike.
pub struct WaterslideTheme {
    pub accent: Color32,
    pub text: Color32,
    pub dim: Color32,
    pub legend: Color32,
    pub screen_bg: Color32,
    pub grid: Color32,
    pub grid_mid: Color32,
    pub cmap: [Color32; 256], // intensity 0..1 → colour
}

/// 6-stop colormap builder. Stops are (position 0..1, [r,g,b]).
pub fn build_cmap(stops: &[(f32, [u8; 3])]) -> [Color32; 256] {
    let mut lut = [Color32::BLACK; 256];
    for i in 0..256 {
        let v = i as f32 / 255.0;
        let mut a = stops[0];
        let mut b = stops[stops.len() - 1];
        for w in stops.windows(2) {
            if v >= w[0].0 && v <= w[1].0 { a = w[0]; b = w[1]; break; }
        }
        let tt = if (b.0 - a.0).abs() < 1e-6 { 0.0 } else { (v - a.0) / (b.0 - a.0) };
        let lerp = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * tt) as u8;
        lut[i] = Color32::from_rgb(
            lerp(a.1[0], b.1[0]),
            lerp(a.1[1], b.1[1]),
            lerp(a.1[2], b.1[2]),
        );
    }
    lut
}

/// The "Martian / graphite" colormap (dark amber). See the README for the
/// phosphor (green) and spectral (cyan) stop sets.
pub fn martian_cmap() -> [Color32; 256] {
    build_cmap(&[
        (0.00, [8, 6, 4]),
        (0.22, [46, 18, 6]),
        (0.42, [110, 44, 8]),
        (0.60, [186, 86, 12]),
        (0.78, [240, 150, 40]),
        (1.00, [255, 236, 205]),
    ])
}

pub struct WaterslidePanel {
    sim: Sim,
    image: ColorImage, // W×H spectrogram, scrolled in place
    tex: Option<TextureHandle>,
    dx_frac: f64,
    primed: bool,
}

impl WaterslidePanel {
    pub fn new(start_t: f64) -> Self {
        Self {
            sim: Sim::new(start_t),
            image: ColorImage::new([W, H], Color32::BLACK),
            tex: None,
            dx_frac: 0.0,
            primed: false,
        }
    }

    fn write_col(&mut self, cx: usize, ts: f64, cmap: &[Color32; 256]) {
        let mut vals = [0f32; COL_H];
        let col_id = (ts * PPS).round() as i64;
        column_values(ts, col_id, &mut vals);
        for y in 0..H {
            let v = vals[y].clamp(0.0, 1.0);
            self.image.pixels[y * W + cx] = cmap[(v * 255.0) as usize];
        }
    }

    fn prime(&mut self, cmap: &[Color32; 256]) {
        let t = self.sim.t();
        for cx in 0..W {
            self.write_col(cx, t - cx as f64 / PPS, cmap);
        }
        self.primed = true;
    }

    fn scroll(&mut self, dt: f64, cmap: &[Color32; 256]) {
        self.dx_frac += dt * PPS;
        let mut dx = self.dx_frac.floor() as usize;
        if dx == 0 { return; }
        if dx > W { dx = W; }
        self.dx_frac -= dx as f64;
        // shift every row right by dx (memmove-safe), newest goes in at the left
        for y in 0..H {
            let row = y * W;
            self.image.pixels.copy_within(row..row + (W - dx), row + dx);
        }
        let t = self.sim.t();
        for cx in 0..dx {
            self.write_col(cx, t - cx as f64 / PPS, cmap);
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, dt: f64, theme: &WaterslideTheme) {
        // 1) advance model + spectrogram
        self.sim.advance(dt);
        if !self.primed { self.prime(&theme.cmap); } else { self.scroll(dt, &theme.cmap); }

        // 2) upload texture
        let img = self.image.clone();
        match &mut self.tex {
            Some(t) => t.set(img, TextureOptions::LINEAR),
            None => self.tex = Some(ui.ctx().load_texture("waterslide", img, TextureOptions::LINEAR)),
        }
        let tex = self.tex.as_ref().unwrap();

        // 3) geometry: map design space (0..DESIGN_W, 0..REF_H) into the pane rect
        let r = ui.available_rect_before_wrap();
        let sx = r.width() / DESIGN_W;
        let sy = r.height() / H as f32;
        let p = |x: f32, y: f32| Pos2::new(r.left() + x * sx, r.top() + y * sy);

        let painter = ui.painter_at(r);
        painter.rect_filled(r, 0.0, theme.screen_bg);

        // FFT image (newest column at its left edge == panel centre)
        let fft_rect = Rect::from_min_max(p(X_FFT_L, 0.0), p(X_FFT_R, H as f32));
        let uv = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0));
        painter.image(tex.id(), fft_rect, uv, Color32::WHITE);

        // graticule: faint horizontal frequency lines across text + fft
        for off in [1500, 1000, 500, 0, -500, -1000, -1500] {
            let y = 250.0 - off as f32 / 3000.0 * H as f32;
            let (col, wdt) = if off == 0 { (theme.grid_mid, 1.4) } else { (theme.grid, 1.0) };
            painter.line_segment([p(0.0, y), p(X_FFT_R, y)], Stroke::new(wdt, col));
        }

        // 4) decoded text lane (left half), clipped, scrolling left
        let text_rect = Rect::from_min_max(p(0.0, 0.0), p(X_TEXT_R, H as f32));
        let tp = ui.painter_at(text_rect);
        let t = self.sim.t();
        for rec in self.sim.records() {
            let x = X_TEXT_R - ((t - rec.td) * PPS) as f32;
            if x < -260.0 { continue; }
            let pos = p(x, rec.final_y);
            let snr_col = if rec.rsnr > -12 { theme.accent } else { theme.dim };
            // message (right-aligned so the newest char sits at "now")
            let msg_w = tp.text(pos, Align2::RIGHT_CENTER, &rec.msg, FontId::monospace(12.0 * sy), theme.text).width();
            // reception SNR just to the left of the message
            let snr_pos = Pos2::new(pos.x - msg_w - 8.0 * sx, pos.y);
            tp.text(snr_pos, Align2::RIGHT_CENTER, sgn(rec.rsnr), FontId::monospace(10.5 * sy), snr_col);
        }

        // 5) NOW divider at the centre
        painter.line_segment([p(521.0, 0.0), p(521.0, H as f32)], Stroke::new(2.0, theme.accent));

        // 6) frequency scale (compact, far right)
        for (off, label) in [
            (1500, "+1500"), (1000, "+1000"), (500, "+500"),
            (0, "14.074"), (-500, "\u{2212}500"), (-1000, "\u{2212}1000"), (-1500, "\u{2212}1500"),
        ] {
            let y = 250.0 - off as f32 / 3000.0 * H as f32;
            let mid = off == 0;
            let col = if mid { theme.accent } else { theme.legend };
            let size = if mid { 12.0 } else { 9.0 } * sy;
            painter.text(p(X_SCALE_L + 6.0, y), Align2::LEFT_CENTER, label, FontId::monospace(size), col);
        }

        ui.ctx().request_repaint(); // keep the animation running
    }
}

fn sgn(n: i32) -> String { format!("{}{:02}", if n <= 0 { "\u{2212}" } else { "+" }, n.abs()) }
