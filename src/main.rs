//! "Martian Hybrid" — FT8 console panel in egui.
//!
//! A single instrument-style panel in the "Martian" theme: brushed-metal chassis,
//! recessed glass screens, amber accent, flat tactical hardware. A fixed-height
//! top bar (identity · clocks · DISPLAY/GUI switches) sits over a resizable body
//! laid out by `egui_tiles` — Waterfall (left) and a right stack of Log Book,
//! Band Scan, and Contacts map. The window and every split are draggable.
//! Layout/geometry/fake-data come from `panel_data`; all colour/chrome flows
//! through a `theme::Palette`, so the light/dark flip is one struct swap.

mod geo_data;
mod panel_data;
mod theme;
mod waterslide_panel;
mod waterslide_sim;

use eframe::egui;
use egui::{
    Align2, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Mesh, Pos2, Rect, Shape,
    Stroke, StrokeKind, TextureHandle, TextureOptions, Vec2,
};
use egui_tiles::{Behavior, Container, Tile, TileId, Tiles, Tree, UiResponse};

use panel_data as pd;
use theme::*;
use waterslide_panel::{WaterslidePanel, WaterslideTheme};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([pd::PANEL_W, pd::PANEL_H])
            .with_min_inner_size([720.0, 460.0])
            .with_title("Martian Hybrid — FT8 Console"),
        ..Default::default()
    };
    eframe::run_native(
        "martian_hybrid",
        options,
        Box::new(|cc| {
            install_fonts(&cc.egui_ctx);
            Ok(Box::new(App::new()))
        }),
    )
}

// ---------------------------------------------------------------------------
// Fonts
// ---------------------------------------------------------------------------

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        "chakra".into(),
        FontData::from_static(include_bytes!("../assets/fonts/ChakraPetch-SemiBold.ttf")).into(),
    );
    fonts.font_data.insert(
        "chakra_bold".into(),
        FontData::from_static(include_bytes!("../assets/fonts/ChakraPetch-Bold.ttf")).into(),
    );
    fonts.font_data.insert(
        "plex".into(),
        FontData::from_static(include_bytes!("../assets/fonts/IBMPlexMono-Medium.ttf")).into(),
    );

    // Two heading families so the design's 600 vs 700 weights stay distinct:
    // legends/headers use Chakra SemiBold; callsigns/numerals/clocks use Bold.
    fonts
        .families
        .insert(FontFamily::Name("heading".into()), vec!["chakra".into()]);
    fonts.families.insert(
        FontFamily::Name("heading_bold".into()),
        vec!["chakra_bold".into()],
    );
    // All data/body text -> Monospace remapped to IBM Plex Mono.
    fonts
        .families
        .insert(FontFamily::Monospace, vec!["plex".into()]);
    // Keep Chakra as the proportional default too, so stray egui widgets match.
    fonts.families.insert(
        FontFamily::Proportional,
        vec!["chakra".into(), "plex".into()],
    );

    ctx.set_fonts(fonts);
}

// ---------------------------------------------------------------------------
// egui widget visuals derived from the active palette
// ---------------------------------------------------------------------------

fn apply_visuals(ctx: &egui::Context, pal: &Palette) {
    let mut v = if pal.is_dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    v.panel_fill = pal.face_bottom;
    v.window_fill = pal.face_bottom;
    v.extreme_bg_color = pal.screen_bg;
    v.override_text_color = Some(pal.body);
    ctx.set_visuals(v);
}

// ---------------------------------------------------------------------------
// Chassis: brushed-metal texture + gradient face.
// ---------------------------------------------------------------------------

fn make_brushed(ctx: &egui::Context, pal: &Palette) -> TextureHandle {
    // One light column, one dark column => 2px stripe period when tiled.
    let img = egui::ColorImage::new([2, 1], vec![pal.stripe_light, pal.stripe_dark]);
    ctx.load_texture("brushed", img, TextureOptions::NEAREST_REPEAT)
}

/// Shaded-relief texture (grayscale multiplier) baked from GEBCO; see
/// `tools/gen_relief.py`. Sampled by the land mesh to give the map topographic
/// depth. Theme-independent — load once.
fn make_relief(ctx: &egui::Context) -> TextureHandle {
    let bytes = include_bytes!("../assets/relief.png");
    let gray = image::load_from_memory(bytes)
        .expect("decode relief.png")
        .to_luma8();
    let (w, h) = gray.dimensions();
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for p in gray.pixels() {
        let v = p[0];
        rgba.extend_from_slice(&[v, v, v, 255]);
    }
    let img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
    ctx.load_texture("relief", img, TextureOptions::LINEAR)
}

/// Paint the chassis: vertical face gradient, then the translucent brushed
/// stripes tiled over it.
fn paint_chassis(painter: &egui::Painter, rect: Rect, pal: &Palette, brushed: &TextureHandle) {
    vertical_gradient(painter, rect, pal.face_top, pal.face_bottom);
    let mut mesh = Mesh::with_texture(brushed.id());
    let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(rect.width() / 2.0, 1.0));
    mesh.add_rect_with_uv(rect, uv, Color32::WHITE);
    painter.add(Shape::mesh(mesh));
}

// ---------------------------------------------------------------------------
// Small chrome helpers
// ---------------------------------------------------------------------------

/// Measure rendered text width (for hand-laying labels).
fn measure(painter: &egui::Painter, text: &str, font: egui::FontId) -> f32 {
    painter
        .layout_no_wrap(text.to_owned(), font, Color32::WHITE)
        .size()
        .x
}

fn shadow(pal: &Palette) -> Color32 {
    if pal.is_dark {
        Color32::from_rgba_unmultiplied(0, 0, 0, 140)
    } else {
        Color32::from_rgba_unmultiplied(255, 255, 255, 120)
    }
}

fn clearc() -> Color32 {
    Color32::from_rgba_unmultiplied(0, 0, 0, 0)
}

/// A recessed LCD surface (clock chips, switch tracks, Scan track): lcd gradient
/// + a short top inset shadow + a 1px edge ring. No inset-shadow primitive in
/// egui, so we fake the bevel.
fn lcd_panel(painter: &egui::Painter, rect: Rect, pal: &Palette, radius: u8) {
    vertical_gradient(painter, rect, pal.lcd_top, pal.lcd_bottom);
    let sh_h = (rect.height() * 0.5).min(9.0);
    let shade = Rect::from_min_size(rect.min, Vec2::new(rect.width(), sh_h));
    let dark = Color32::from_rgba_unmultiplied(0, 0, 0, if pal.is_dark { 130 } else { 70 });
    vertical_gradient(painter, shade, dark, clearc());
    painter.rect_stroke(
        rect,
        corner_radius(radius),
        Stroke::new(1.0, pal.edge),
        StrokeKind::Inside,
    );
}

/// One segmented-control key: lit accent fill + raised highlight when active,
/// transparent when inactive. Returns the click response.
fn key_cell(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    pal: &Palette,
    rect: Rect,
    label: &str,
    active: bool,
    id: egui::Id,
) -> egui::Response {
    if active {
        painter.rect_filled(rect, corner_radius(2), pal.accent);
        // raised: 1px top highlight + 1px bottom shadow.
        let hl = Color32::from_rgba_unmultiplied(255, 255, 255, 72);
        let sh = Color32::from_rgba_unmultiplied(0, 0, 0, 115);
        painter.line_segment(
            [
                Pos2::new(rect.left() + 1.5, rect.top() + 0.75),
                Pos2::new(rect.right() - 1.5, rect.top() + 0.75),
            ],
            Stroke::new(1.0, hl),
        );
        painter.line_segment(
            [
                Pos2::new(rect.left() + 1.5, rect.bottom() - 0.5),
                Pos2::new(rect.right() - 1.5, rect.bottom() - 0.5),
            ],
            Stroke::new(1.0, sh),
        );
    }
    let (font, color) = if active {
        (heading_bold(9.0), pal.on_accent)
    } else {
        (heading(9.0), pal.sub)
    };
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        &tracked(label),
        font,
        color,
    );
    ui.interact(rect, id, egui::Sense::click())
}

/// Draw a dashed polyline, keeping dash phase across segment joints.
fn dashed_polyline(painter: &egui::Painter, pts: &[Pos2], stroke: Stroke, dash: f32, gap: f32) {
    let mut drawing = true;
    let mut remaining = dash;
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let seg = b - a;
        let len = seg.length();
        if len < 1e-4 {
            continue;
        }
        let dir = seg / len;
        let mut pos = 0.0;
        let mut start = a;
        while pos < len {
            let step = remaining.min(len - pos);
            let end = a + dir * (pos + step);
            if drawing {
                painter.line_segment([start, end], stroke);
            }
            pos += step;
            remaining -= step;
            start = end;
            if remaining <= 1e-4 {
                drawing = !drawing;
                remaining = if drawing { dash } else { gap };
            }
        }
    }
}

fn ellipse_pts(center: Pos2, rx: f32, ry: f32, n: usize) -> Vec<Pos2> {
    (0..=n)
        .map(|i| {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            Pos2::new(center.x + rx * a.cos(), center.y + ry * a.sin())
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Shared panel chrome: header (spine + legend + sub) + the standard block split.
// ---------------------------------------------------------------------------

/// Draw a panel header (spine + uppercase legend + sub-label).
fn panel_header(painter: &egui::Painter, header: Rect, pal: &Palette, title: &str, sub: &str) {
    let cy = header.center().y;
    let after = spine(painter, Pos2::new(header.left(), cy), pal.accent);
    let tx = after + 8.0;
    let t = tracked(&title.to_uppercase());
    engraved_text(
        painter,
        Pos2::new(tx, cy),
        &t,
        heading(11.0),
        pal.legend,
        shadow(pal),
        Align2::LEFT_CENTER,
    );
    if !sub.is_empty() {
        let sub_x = tx + measure(painter, &t, heading(11.0)) + 10.0;
        painter.text(
            Pos2::new(sub_x, cy),
            Align2::LEFT_CENTER,
            sub,
            mono(8.5),
            pal.sub,
        );
    }
}

/// Split a panel block into (header row, recessed screen) per the standard
/// 24px header + 6px gap + screen recipe.
fn split_block(block: Rect) -> (Rect, Rect) {
    let header = Rect::from_min_max(
        block.min,
        Pos2::new(block.right(), block.top() + pd::HEADER_ROW_H),
    );
    let screen = Rect::from_min_max(
        Pos2::new(block.left(), header.bottom() + pd::HEADER_GAP),
        block.max,
    );
    (header, screen)
}

// =====================================================================
// Pane bodies (free functions so the tile Behavior can call them).
// =====================================================================

/// Waterfall pane: header (● SPLIT AGC) + live Waterslide sim + ticker row.
fn draw_waterfall(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    block: Rect,
    pal: &Palette,
    waterslide: &mut WaterslidePanel,
    dt: f64,
) {
    let header = Rect::from_min_max(
        block.min,
        Pos2::new(block.right(), block.top() + pd::HEADER_ROW_H),
    );
    panel_header(painter, header, pal, "FT8", "0–3000 Hz · time → left");
    // right side: prominent tuned-frequency readout
    let cy = header.center().y;
    let mut rx = header.right() - 2.0;
    painter.text(Pos2::new(rx, cy), Align2::RIGHT_CENTER, "MHz", mono(8.5), pal.sub);
    rx -= measure(painter, "MHz", mono(8.5)) + 5.0;
    engraved_text(
        painter,
        Pos2::new(rx, cy),
        "14.074",
        heading_bold(15.0),
        pal.accent,
        shadow(pal),
        Align2::RIGHT_CENTER,
    );

    // ticker (bottom) + screen (fills between header and ticker).
    let ticker = Rect::from_min_max(
        Pos2::new(block.left(), block.bottom() - pd::TICKER_H),
        block.max,
    );
    let screen = Rect::from_min_max(
        Pos2::new(block.left(), header.bottom() + pd::HEADER_GAP),
        Pos2::new(block.right(), ticker.top() - pd::GAP),
    );
    recessed_screen(painter, screen, pal);

    // Live Waterslide simulation as the screen body (inset to keep brackets).
    if screen.width() > 24.0 && screen.height() > 24.0 {
        let body = screen.shrink(8.0);
        let theme = WaterslideTheme::from_palette(pal);
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(body)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        child.set_clip_rect(screen.shrink(2.0));
        waterslide.ui(&mut child, body, dt, &theme);
    }

    draw_ticker(painter, ticker, pal);
}

fn draw_ticker(painter: &egui::Painter, rect: Rect, pal: &Palette) {
    let cy = rect.center().y;
    let segments: [(&str, Color32, egui::FontId); 4] = [
        ("235945", pal.sub, mono(9.0)),
        ("  −02  ", pal.accent, heading_bold(9.0)),
        ("K1ABC W9XYZ EM48", pal.legend, mono(9.0)),
        ("  ·  235945 −17 CQ EA7KW IM67", pal.sub, mono(9.0)),
    ];
    let painter = painter.with_clip_rect(rect);
    let mut x = rect.left();
    for (text, color, font) in segments {
        let w = measure(&painter, text, font.clone());
        painter.text(Pos2::new(x, cy), Align2::LEFT_CENTER, text, font, color);
        x += w;
    }
}

/// Log Book pane: 4-row QSO table.
fn draw_log(painter: &egui::Painter, block: Rect, pal: &Palette) {
    let (header, screen) = split_block(block);
    panel_header(painter, header, pal, "Log Book", "last 4 · FT8");
    painter.text(
        Pos2::new(header.right() - 2.0, header.center().y),
        Align2::RIGHT_CENTER,
        "312 QSO",
        heading(9.0),
        pal.legend,
    );
    recessed_screen(painter, screen, pal);

    let l = screen.left() + 12.0;
    let r = screen.right() - 12.0;
    if r <= l {
        return;
    }
    let x_call = l + 50.0;
    let x_grid = r - 48.0 - 48.0 - 60.0;
    let x_snt = r - 48.0; // right edge of Snt column
    let x_rcv = r; // right edge of Rcv column

    let hy = screen.top() + 10.0;
    let dimf = mono(8.0);
    for (text, x, align) in [
        ("UTC", l, Align2::LEFT_CENTER),
        ("CALL", x_call, Align2::LEFT_CENTER),
        ("GRID", x_grid, Align2::LEFT_CENTER),
        ("SNT", x_snt, Align2::RIGHT_CENTER),
        ("RCV", x_rcv, Align2::RIGHT_CENTER),
    ] {
        painter.text(Pos2::new(x, hy), align, &tracked(text), dimf.clone(), pal.dim);
    }
    let sep_y = screen.top() + 19.0;
    painter.line_segment(
        [Pos2::new(l, sep_y), Pos2::new(r, sep_y)],
        Stroke::new(1.0, pal.dim.gamma_multiply(0.4)),
    );

    for (i, (utc, call, grid, snt, rcv)) in pd::LOGS.iter().enumerate() {
        let ry = sep_y + 11.0 + i as f32 * 22.0;
        painter.text(Pos2::new(l, ry), Align2::LEFT_CENTER, utc, mono(10.0), pal.dim);
        painter.text(
            Pos2::new(x_call, ry),
            Align2::LEFT_CENTER,
            &tracked(call),
            heading(10.0),
            pal.body,
        );
        painter.text(Pos2::new(x_grid, ry), Align2::LEFT_CENTER, grid, mono(10.0), pal.dim);
        painter.text(Pos2::new(x_snt, ry), Align2::RIGHT_CENTER, snt, mono(10.0), pal.body);
        painter.text(Pos2::new(x_rcv, ry), Align2::RIGHT_CENTER, rcv, mono(10.0), pal.accent);
        if i + 1 < pd::LOGS.len() {
            let ly = ry + 11.0;
            painter.line_segment(
                [Pos2::new(l, ly), Pos2::new(r, ly)],
                Stroke::new(1.0, pal.dim.gamma_multiply(0.22)),
            );
        }
    }
}

/// Band Scan pane: two-column band blocks + a Scan/Cancel lit key.
fn draw_band_scan(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    block: Rect,
    pal: &Palette,
    scan: &mut Scan,
) {
    let (header, screen) = split_block(block);

    let status = if scan.running {
        format!("Scanning {} …", pd::BANDS[scan.active_band].0)
    } else if scan.last_min == 0 {
        "Last scan: just now".to_string()
    } else {
        format!("Last scan: {} min ago", scan.last_min)
    };
    panel_header(painter, header, pal, "Band Scan", &status);

    // Scan / Cancel button (lit accent key in a recessed track), header-right.
    let label = if scan.running { "CANCEL" } else { "SCAN" };
    let cy = header.center().y;
    let cell_w = measure(painter, &tracked(label), heading_bold(9.0)) + 22.0;
    let track_w = cell_w + 4.0;
    let track = Rect::from_min_max(
        Pos2::new(header.right() - track_w, cy - 11.0),
        Pos2::new(header.right(), cy + 11.0),
    );
    lcd_panel(painter, track, pal, 4);
    let cell = Rect::from_min_max(
        Pos2::new(track.left() + 2.0, track.top() + 2.0),
        Pos2::new(track.right() - 2.0, track.bottom() - 2.0),
    );
    let resp = key_cell(ui, painter, pal, cell, label, true, ui.id().with("scan_btn"));
    if resp.clicked() {
        if scan.running {
            scan.running = false;
            scan.accum = 0.0;
        } else {
            scan.running = true;
            scan.active_band = 0;
            scan.accum = 0.0;
        }
    }

    recessed_screen(painter, screen, pal);

    // Two columns split by a 1px divider; left = [40m,20m], right = [15m,10m].
    let mid = screen.center().x;
    painter.line_segment(
        [
            Pos2::new(mid, screen.top() + 8.0),
            Pos2::new(mid, screen.bottom() - 8.0),
        ],
        Stroke::new(1.0, pal.dim.gamma_multiply(0.4)),
    );
    let left_half = Rect::from_min_max(screen.min, Pos2::new(mid, screen.bottom()));
    let right_half = Rect::from_min_max(Pos2::new(mid, screen.top()), screen.max);
    draw_band_column(painter, left_half, pal, scan, &[0, 1]);
    draw_band_column(painter, right_half, pal, scan, &[2, 3]);
}

fn draw_band_column(painter: &egui::Painter, half: Rect, pal: &Palette, scan: &Scan, idxs: &[usize; 2]) {
    const BLOCK_H: f32 = 30.0;
    const BLOCK_GAP: f32 = 7.0;
    let total = BLOCK_H * 2.0 + BLOCK_GAP;
    let top = half.center().y - total / 2.0;
    let content_left = half.left() + 12.0;

    for (slot, &bi) in idxs.iter().enumerate() {
        let (band, heard, unworked) = pd::BANDS[bi];
        let active = scan.running && scan.active_band == bi;
        let by = top + slot as f32 * (BLOCK_H + BLOCK_GAP);
        let bcy = by + BLOCK_H / 2.0;

        if active {
            painter.rect_filled(
                Rect::from_min_max(
                    Pos2::new(content_left, by),
                    Pos2::new(content_left + 2.0, by + BLOCK_H),
                ),
                CornerRadius::ZERO,
                pal.accent,
            );
        }
        let num_x = content_left + 10.0;
        let num_color = if active { pal.accent } else { pal.sub };
        painter.text(
            Pos2::new(num_x, bcy),
            Align2::LEFT_CENTER,
            band,
            heading_bold(22.0),
            num_color,
        );

        let text_x = num_x + 40.0 + 9.0;
        let n1 = format!("{heard}");
        let w1 = painter
            .text(Pos2::new(text_x, bcy - 7.0), Align2::LEFT_CENTER, &n1, mono(11.0), pal.legend)
            .width();
        painter.text(
            Pos2::new(text_x + w1 + 3.0, bcy - 7.0),
            Align2::LEFT_CENTER,
            "heard",
            mono(11.0),
            pal.dim,
        );
        let n2 = format!("{unworked}");
        let w2 = painter
            .text(Pos2::new(text_x, bcy + 7.0), Align2::LEFT_CENTER, &n2, mono(11.0), pal.accent)
            .width();
        painter.text(
            Pos2::new(text_x + w2 + 3.0, bcy + 7.0),
            Align2::LEFT_CENTER,
            "unworked",
            mono(11.0),
            pal.dim,
        );
    }
}

/// Contacts pane: North-America map + the toggle/SNR footer along the bottom.
fn draw_contacts(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    block: Rect,
    pal: &Palette,
    toggles: &mut [bool; 4],
    relief: &TextureHandle,
) {
    let header = Rect::from_min_max(
        block.min,
        Pos2::new(block.right(), block.top() + pd::HEADER_ROW_H),
    );
    panel_header(painter, header, pal, "Contacts", "N. America · DN70KA");
    painter.text(
        Pos2::new(header.right() - 2.0, header.center().y),
        Align2::RIGHT_CENTER,
        &format!("{} spots", pd::WORKED.len()),
        mono(8.5),
        pal.sub,
    );

    let footer = Rect::from_min_max(
        Pos2::new(block.left(), block.bottom() - pd::FOOTER_H),
        block.max,
    );
    let screen = Rect::from_min_max(
        Pos2::new(block.left(), header.bottom() + pd::HEADER_GAP),
        Pos2::new(block.right(), footer.top() - pd::GAP),
    );
    recessed_screen(painter, screen, pal);
    draw_map(painter, screen, pal, relief);
    draw_footer(ui, painter, footer, pal, toggles);
}

/// Composite a translucent foreground over an opaque background → opaque color.
fn over(fg: Color32, bg: Color32) -> Color32 {
    let a = fg.a() as f32 / 255.0;
    let m = |f: u8, b: u8| (f as f32 * a + b as f32 * (1.0 - a)).round() as u8;
    Color32::from_rgb(m(fg.r(), bg.r()), m(fg.g(), bg.g()), m(fg.b(), bg.b()))
}

fn draw_map(painter: &egui::Painter, screen: Rect, pal: &Palette, relief: &TextureHandle) {
    if screen.width() < 24.0 || screen.height() < 24.0 {
        return;
    }
    // SVG content area: padding t6 r8 b4 l8.
    let content = Rect::from_min_max(
        Pos2::new(screen.left() + 8.0, screen.top() + 6.0),
        Pos2::new(screen.right() - 8.0, screen.bottom() - 4.0),
    );

    // Dynamic bounds: fit the box (in world/SVG units) spanning every plotted
    // station plus home. Home is included but not centered, so it lands wherever
    // the worked cluster puts it (e.g. contacts to the west → home biased right).
    let mut pts: Vec<Vec2> = pd::WORKED
        .iter()
        .filter_map(|s| pd::station_lonlat(s.call, s.grid))
        .map(|(lon, lat)| Vec2::new(pd::map_x(lon), pd::map_y(lat)))
        .collect();
    pts.push(Vec2::new(pd::map_x(pd::HOME_LON), pd::map_y(pd::HOME_LAT)));
    let (mut minx, mut miny, mut maxx, mut maxy) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for v in &pts {
        minx = minx.min(v.x); miny = miny.min(v.y);
        maxx = maxx.max(v.x); maxy = maxy.max(v.y);
    }
    // Pad ~8% and guard against a degenerate (single-point) box.
    let bw = (maxx - minx).max(1.0);
    let bh = (maxy - miny).max(1.0);
    minx -= bw * 0.08; maxx += bw * 0.08;
    miny -= bh * 0.08; maxy += bh * 0.08;
    let (bcx, bcy) = ((minx + maxx) * 0.5, (miny + maxy) * 0.5);
    let scale = (content.width() / (maxx - minx)).min(content.height() / (maxy - miny));
    let p = |sx: f32, sy: f32| {
        Pos2::new(content.center().x + (sx - bcx) * scale, content.center().y + (sy - bcy) * scale)
    };
    let proj = |lon: f32, lat: f32| p(pd::map_x(lon), pd::map_y(lat));
    let sl = |v: f32| v * scale; // svg length -> px
    let font = |sz: f32| mono(sz * scale);

    let map_painter = painter.with_clip_rect(screen.shrink(2.0));
    let painter = &map_painter;

    // 1) basemap: pre-triangulated land + lakes (Natural Earth 50m, earcut offline).
    let project = |verts: &[(f32, f32)]| -> Vec<Pos2> {
        verts.iter().map(|&(la, lo)| proj(lo, la)).collect()
    };
    let stroke_rings = |pos: &[Pos2], rings: &[(u32, u32)], stroke: Stroke| {
        for &(s, l) in rings {
            let ring = &pos[s as usize..(s + l) as usize];
            let mut closed = ring.to_vec();
            closed.push(ring[0]);
            painter.add(Shape::line(closed, stroke));
        }
    };

    // Land fill is a textured mesh: each vertex carries a UV into the shaded-relief
    // texture, and an opaque base tint that the relief texel multiplies — so the
    // land lightness varies with terrain (mountains shaded, plains flat).
    let land_base = over(pal.map_land, pal.screen_bg);
    let lon_span = pd::RELIEF_LON1 - pd::RELIEF_LON0;
    let lat_span = pd::RELIEF_LAT1 - pd::RELIEF_LAT0;
    let land_pos = project(geo_data::LAND_VERTS);
    let mut land_mesh = Mesh::with_texture(relief.id());
    for (i, &(la, lo)) in geo_data::LAND_VERTS.iter().enumerate() {
        let uv = Pos2::new((lo - pd::RELIEF_LON0) / lon_span, (pd::RELIEF_LAT1 - la) / lat_span);
        land_mesh.vertices.push(egui::epaint::Vertex { pos: land_pos[i], uv, color: land_base });
    }
    land_mesh.indices.extend_from_slice(geo_data::LAND_IDX);
    painter.add(Shape::mesh(land_mesh));
    stroke_rings(&land_pos, geo_data::LAND_RINGS, Stroke::new(sl(0.5).max(0.6), pal.map_coast));

    // Lakes: translucent dark fill punches the land back down to water tone.
    let lake_fill = Color32::from_rgba_unmultiplied(pal.screen_bg.r(), pal.screen_bg.g(), pal.screen_bg.b(), 220);
    let lake_pos = project(geo_data::LAKES_VERTS);
    let mut lake_mesh = Mesh::default();
    for pos in &lake_pos {
        lake_mesh.colored_vertex(*pos, lake_fill);
    }
    for t in geo_data::LAKES_IDX.chunks_exact(3) {
        lake_mesh.add_triangle(t[0], t[1], t[2]);
    }
    painter.add(Shape::mesh(lake_mesh));
    stroke_rings(&lake_pos, geo_data::LAKES_RINGS, Stroke::new(sl(0.4).max(0.5), pal.map_coast.gamma_multiply(0.7)));

    // 2) graticule
    let grat = pal.dim.gamma_multiply(0.25);
    for &lon in pd::MERIDIANS {
        let x = pd::map_x(lon);
        painter.line_segment([p(x, 0.0), p(x, pd::MAP_H)], Stroke::new(0.4, grat));
    }
    for &lat in pd::PARALLELS {
        let y = pd::map_y(lat);
        painter.line_segment([p(0.0, y), p(pd::MAP_W, y)], Stroke::new(0.4, grat));
        painter.text(
            p(2.0, y - 1.5),
            Align2::LEFT_BOTTOM,
            &format!("{lat:.0}°"),
            font(4.6),
            pal.dim.gamma_multiply(0.65),
        );
    }

    // 3) 49°N border (dashed accent), left edge → lon −95
    let by = pd::map_y(pd::BORDER_LAT);
    dashed_polyline(
        painter,
        &[p(0.0, by), p(pd::map_x(pd::BORDER_LON_END), by)],
        Stroke::new(sl(0.5).max(0.6), pal.accent.gamma_multiply(0.5)),
        sl(3.0),
        sl(2.0),
    );

    // 4) range rings (dashed ellipses about home)
    let home = proj(pd::HOME_LON, pd::HOME_LAT);
    for &km in pd::RING_KM {
        let rx = sl((km / 85.0) * pd::KX * pd::S);
        let ry = sl((km / 111.0) * pd::S);
        let pts = ellipse_pts(home, rx, ry, 96);
        dashed_polyline(
            painter,
            &pts,
            Stroke::new(sl(0.45).max(0.6), pal.accent.gamma_multiply(0.32)),
            sl(2.0),
            sl(2.5),
        );
    }

    // 5) worked spots (filled) — position inferred from each station's grid.
    // Marker/label sized in px (with clamp) so they stay readable at any zoom.
    let spot_r = sl(2.4).clamp(2.0, 3.6);
    let label_font = mono(sl(4.8).clamp(5.0, 8.0));
    for s in pd::WORKED {
        let Some((lon, lat)) = pd::station_lonlat(s.call, s.grid) else { continue };
        let pos = proj(lon, lat);
        painter.circle_filled(pos, spot_r, pal.accent);
        // Flip the label to the inboard side near the right/top edges so it stays on-screen.
        let right = pos.x > content.right() - 42.0;
        let near_top = pos.y < content.top() + 12.0;
        let off = Vec2::new(
            if right { -(spot_r + 1.5) } else { spot_r + 1.5 },
            if near_top { spot_r + 5.0 } else { -(spot_r + 1.0) },
        );
        let align = if right { Align2::RIGHT_BOTTOM } else { Align2::LEFT_BOTTOM };
        painter.text(pos + off, align, s.call, label_font.clone(), pal.body);
    }

    // 6) home / QTH marker — the strongest indicator, drawn last so it sits on top.
    let ring_r = sl(4.6).clamp(5.0, 7.0);
    let arm = ring_r + 2.5;
    painter.circle(home, ring_r, Color32::TRANSPARENT, Stroke::new(1.4, pal.accent));
    painter.line_segment([Pos2::new(home.x - arm, home.y), Pos2::new(home.x + arm, home.y)], Stroke::new(1.0, pal.accent));
    painter.line_segment([Pos2::new(home.x, home.y - arm), Pos2::new(home.x, home.y + arm)], Stroke::new(1.0, pal.accent));
    painter.circle_filled(home, (spot_r + 0.8).max(2.6), pal.accent);
    painter.text(Pos2::new(home.x + arm, home.y - arm), Align2::LEFT_BOTTOM, "QTH", heading(sl(4.8).clamp(6.0, 9.0)), pal.accent);
}

/// Flat tactical footer: square toggles (solid = on, hollow = off) + SNR bars.
fn draw_footer(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    rect: Rect,
    pal: &Palette,
    toggles: &mut [bool; 4],
) {
    let cy = rect.center().y;
    let labels = ["DX ONLY", "CQ", "ALERT", "LOG"];
    let mut x = rect.left();
    for i in 0..4 {
        let sq = Rect::from_center_size(Pos2::new(x + TOGGLE_SQ * 0.5, cy), Vec2::splat(TOGGLE_SQ));
        let resp = ui.interact(
            sq.expand(2.0),
            ui.id().with(("footer_toggle", i)),
            egui::Sense::click(),
        );
        if resp.clicked() {
            toggles[i] = !toggles[i];
        }
        if toggles[i] {
            painter.rect_filled(sq, CornerRadius::ZERO, pal.accent);
        } else {
            painter.rect_stroke(
                sq,
                CornerRadius::ZERO,
                Stroke::new(TOGGLE_STROKE, pal.sub),
                StrokeKind::Inside,
            );
        }
        let label_color = if toggles[i] { pal.legend } else { pal.sub };
        let tx = sq.right() + 6.0;
        let label = tracked(labels[i]);
        painter.text(Pos2::new(tx, cy), Align2::LEFT_CENTER, &label, heading(8.5), label_color);
        x = tx + measure(painter, &label, heading(8.5)) + 18.0;
    }

    // SNR bar-graph (right).
    let heights = [5.0, 8.0, 11.0, 14.0, 9.0, 5.0];
    let mut bx = rect.right() - 70.0;
    let base = cy + 7.0;
    for (j, h) in heights.iter().enumerate() {
        let on = j < 4;
        let bar = Rect::from_min_max(Pos2::new(bx, base - h), Pos2::new(bx + 3.0, base));
        let col = if on { pal.accent } else { pal.sub.gamma_multiply(0.45) };
        painter.rect_filled(bar, CornerRadius::ZERO, col);
        bx += 5.0;
    }
    painter.text(Pos2::new(bx + 4.0, cy), Align2::LEFT_CENTER, "SNR", mono(7.5), pal.sub);
}

// =====================================================================
// Tile tree + Behavior
// =====================================================================

#[derive(Clone, Copy, PartialEq)]
enum PaneKind {
    Waterfall,
    Log,
    BandScan,
    Contacts,
}

struct Pane {
    title: String,
    kind: PaneKind,
}

impl Pane {
    fn new(title: &str, kind: PaneKind) -> Self {
        Self {
            title: title.into(),
            kind,
        }
    }
}

/// Per-frame view handed to the tile behavior: active palette + the mutable UI
/// state a pane may need to read or write.
struct Tactical<'a> {
    pal: &'a Palette,
    scan: &'a mut Scan,
    toggles: &'a mut [bool; 4],
    waterslide: &'a mut WaterslidePanel,
    relief: &'a TextureHandle,
    dt: f64,
}

impl<'a> Behavior<Pane> for Tactical<'a> {
    fn pane_ui(&mut self, ui: &mut egui::Ui, _id: TileId, pane: &mut Pane) -> UiResponse {
        // The chassis is already painted behind the whole tree. Inset the pane
        // so the recessed screen has chassis breathing room around it (and the
        // grooves between panes read as metal).
        let block = ui.max_rect().shrink2(Vec2::new(8.0, 6.0));
        let painter = ui.painter().clone();
        let pal = self.pal;
        match pane.kind {
            PaneKind::Waterfall => {
                draw_waterfall(ui, &painter, block, pal, self.waterslide, self.dt)
            }
            PaneKind::Log => draw_log(&painter, block, pal),
            PaneKind::BandScan => draw_band_scan(ui, &painter, block, pal, self.scan),
            PaneKind::Contacts => draw_contacts(ui, &painter, block, pal, self.toggles, self.relief),
        }
        UiResponse::None
    }

    fn tab_title_for_pane(&mut self, pane: &Pane) -> egui::WidgetText {
        pane.title.clone().into()
    }

    // ---- chrome suppression: flatten everything egui_tiles would draw ----

    fn gap_width(&self, _style: &egui::Style) -> f32 {
        pd::VGROOVE_W // grooves: chassis shows through between panes
    }

    fn simplification_options(&self) -> egui_tiles::SimplificationOptions {
        egui_tiles::SimplificationOptions {
            all_panes_must_have_tabs: false,
            ..Default::default()
        }
    }

    fn resize_stroke(&self, _style: &egui::Style, state: egui_tiles::ResizeState) -> Stroke {
        match state {
            egui_tiles::ResizeState::Idle => Stroke::NONE,
            egui_tiles::ResizeState::Hovering => Stroke::new(1.0, self.pal.accent.gamma_multiply(0.5)),
            egui_tiles::ResizeState::Dragging => Stroke::new(2.0, self.pal.accent),
        }
    }
}

fn build_tree() -> Tree<Pane> {
    let mut tiles = Tiles::default();
    let waterfall = tiles.insert_pane(Pane::new("Waterfall", PaneKind::Waterfall));
    let log = tiles.insert_pane(Pane::new("Log Book", PaneKind::Log));
    let band = tiles.insert_pane(Pane::new("Band Scan", PaneKind::BandScan));
    let contacts = tiles.insert_pane(Pane::new("Contacts", PaneKind::Contacts));

    let right = tiles.insert_vertical_tile(vec![log, band, contacts]);
    // Initial right-column proportions ≈ the design heights (Log 142, Band 112,
    // Contacts fills the rest). Resizable from here.
    if let Some(Tile::Container(Container::Linear(lin))) = tiles.get_mut(right) {
        lin.shares.set_share(log, pd::LOG_H);
        lin.shares.set_share(band, pd::BANDSCAN_H);
        lin.shares.set_share(contacts, pd::PANEL_H - pd::LOG_H - pd::BANDSCAN_H);
    }

    let root = tiles.insert_horizontal_tile(vec![waterfall, right]);
    if let Some(Tile::Container(Container::Linear(lin))) = tiles.get_mut(root) {
        lin.shares.set_share(waterfall, pd::LEFT_COL_W);
        lin.shares.set_share(right, pd::PANEL_W - pd::LEFT_COL_W);
    }
    Tree::new("martian_tree", root, tiles)
}

// =====================================================================
// App state
// =====================================================================

struct Scan {
    running: bool,
    active_band: usize,
    accum: f32,    // seconds into the current band dwell
    last_min: u32, // "Last scan: N min ago" (0 == just now)
}

struct App {
    dark: bool,
    edit_mode: bool, // GUI LOCK/EDIT
    scan: Scan,
    toggles: [bool; 4], // footer DX ONLY / CQ / ALERT / LOG
    waterslide: WaterslidePanel,
    tree: Tree<Pane>,
    brushed: Option<TextureHandle>,
    brushed_is_dark: bool,
    relief: Option<TextureHandle>,
    visuals_set_for: Option<bool>,
    /// If set (via MARTIAN_SHOT=path), render a few frames, save a PNG, exit.
    shot_path: Option<String>,
    frame: u64,
}

impl App {
    fn new() -> Self {
        let dark = std::env::var("MARTIAN_LIGHT").is_err();
        Self {
            dark,
            edit_mode: false,
            scan: Scan {
                running: false,
                active_band: 0,
                accum: 0.0,
                last_min: 4,
            },
            toggles: [true, false, false, true], // DX ONLY + LOG on, per reference
            waterslide: WaterslidePanel::new(7200.0),
            tree: build_tree(),
            brushed: None,
            brushed_is_dark: !dark,
            relief: None,
            visuals_set_for: None,
            shot_path: std::env::var("MARTIAN_SHOT").ok(),
            frame: 0,
        }
    }

    fn palette(&self) -> Palette {
        if self.dark { GRAPHITE } else { SILVER }
    }
}

const SCAN_DWELL: f32 = 2.5; // seconds per band

impl eframe::App for App {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root.ctx().clone();

        // Work around an upstream macOS/AppKit teardown crash: exit immediately
        // when a close is requested, skipping winit's responder-chain teardown.
        if ctx.input(|i| i.viewport().close_requested()) {
            std::process::exit(0);
        }

        let pal = self.palette();
        if self.visuals_set_for != Some(self.dark) {
            apply_visuals(&ctx, &pal);
            self.visuals_set_for = Some(self.dark);
        }
        if self.brushed.is_none() || self.brushed_is_dark != self.dark {
            self.brushed = Some(make_brushed(&ctx, &pal));
            self.brushed_is_dark = self.dark;
        }
        let brushed = self.brushed.clone().unwrap();
        if self.relief.is_none() {
            self.relief = Some(make_relief(&ctx));
        }
        let relief = self.relief.clone().unwrap();

        let dt = ctx.input(|i| i.stable_dt);
        self.tick_scan(dt);

        // -------- top bar (fixed height) --------
        egui::Panel::top("topbar")
            .exact_size(pd::TOPBAR_H + pd::GROOVE_H)
            .frame(egui::Frame::NONE)
            .show_inside(root, |ui| {
                let painter = ui.painter().clone();
                let rect = ui.max_rect();
                paint_chassis(&painter, rect, &pal, &brushed);
                let bar = Rect::from_min_max(
                    rect.min,
                    Pos2::new(rect.right(), rect.top() + pd::TOPBAR_H),
                );
                self.top_bar(ui, &painter, bar, &pal);
                // groove under the bar
                painter.rect_filled(
                    Rect::from_min_max(
                        Pos2::new(rect.left(), bar.bottom()),
                        Pos2::new(rect.right(), bar.bottom() + pd::GROOVE_H),
                    ),
                    CornerRadius::ZERO,
                    pal.edge,
                );
            });

        // -------- body: chassis + resizable tile tree --------
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(pal.face_bottom))
            .show_inside(root, |ui| {
                let painter = ui.painter().clone();
                paint_chassis(&painter, ui.max_rect(), &pal, &brushed);
                let mut behavior = Tactical {
                    pal: &pal,
                    scan: &mut self.scan,
                    toggles: &mut self.toggles,
                    waterslide: &mut self.waterslide,
                    relief: &relief,
                    dt: dt as f64,
                };
                self.tree.ui(&mut behavior, ui);
            });

        self.run_screenshot(&ctx);
    }
}

impl App {
    fn tick_scan(&mut self, dt: f32) {
        if !self.scan.running {
            return;
        }
        self.scan.accum += dt;
        while self.scan.accum >= SCAN_DWELL {
            self.scan.accum -= SCAN_DWELL;
            self.scan.active_band += 1;
            if self.scan.active_band >= pd::BANDS.len() {
                self.scan.running = false;
                self.scan.active_band = 0;
                self.scan.accum = 0.0;
                self.scan.last_min = 0;
                break;
            }
        }
    }
}

// =====================================================================
// Top bar
// =====================================================================

impl App {
    fn top_bar(&mut self, ui: &mut egui::Ui, painter: &egui::Painter, bar: Rect, pal: &Palette) {
        let cy = bar.center().y;

        // ---- identity (far left): spine 3×16 + N0JDC + DN70KA ----
        let x0 = bar.left() + 14.0;
        let spine = Rect::from_min_max(Pos2::new(x0, cy - 8.0), Pos2::new(x0 + 3.0, cy + 8.0));
        painter.rect_filled(spine, CornerRadius::ZERO, pal.accent);
        let call_x = spine.right() + 8.0;
        let call = tracked("N0JDC");
        engraved_text(
            painter,
            Pos2::new(call_x, cy),
            &call,
            heading_bold(18.0),
            pal.legend,
            shadow(pal),
            Align2::LEFT_CENTER,
        );
        let grid_x = call_x + measure(painter, &call, heading_bold(18.0)) + 9.0;
        painter.text(
            Pos2::new(grid_x, cy + 1.0),
            Align2::LEFT_CENTER,
            &tracked("DN70KA"),
            mono(9.0),
            pal.sub,
        );

        // ---- right cluster, laid out right-to-left ----
        let right_edge = bar.right() - 24.0;

        let (gui_left, gui_clicks) = self.segmented(
            ui,
            painter,
            pal,
            right_edge,
            cy,
            "GUI",
            &[("LOCK", !self.edit_mode), ("EDIT", self.edit_mode)],
            "sw_gui",
        );
        if gui_clicks[0] {
            self.edit_mode = false;
        }
        if gui_clicks[1] {
            self.edit_mode = true;
        }

        let (disp_left, disp_clicks) = self.segmented(
            ui,
            painter,
            pal,
            gui_left - 14.0,
            cy,
            "DISPLAY",
            &[("DARK", self.dark), ("LIGHT", !self.dark)],
            "sw_disp",
        );
        if disp_clicks[0] {
            self.dark = true;
        }
        if disp_clicks[1] {
            self.dark = false;
        }

        // ---- clocks (two LCD chips), to the left of the switches ----
        let utc = format!("{}", chrono::Utc::now().format("%H:%M:%S"));
        let local = format!("{}", chrono::Local::now().format("%H:%M:%S"));
        let utc_left = lcd_clock(painter, pal, disp_left - 16.0, cy, "UTC", &utc);
        let _ = lcd_clock(painter, pal, utc_left - 10.0, cy, "LOCAL", &local);

        // Tick the clocks at least once a second even if nothing animates.
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs(1));
    }

    /// A segmented switch (micro-label above a recessed track of key cells),
    /// flush to `right_x`. Returns its left edge and a per-cell click flag.
    fn segmented(
        &self,
        ui: &mut egui::Ui,
        painter: &egui::Painter,
        pal: &Palette,
        right_x: f32,
        cy: f32,
        micro: &str,
        cells: &[(&str, bool)],
        id_src: &str,
    ) -> (f32, Vec<bool>) {
        const TRACK_H: f32 = 22.0;
        const PAD: f32 = 2.0;
        const GAP: f32 = 2.0;
        const CELL_PAD_X: f32 = 11.0;

        let widths: Vec<f32> = cells
            .iter()
            .map(|(t, _)| measure(painter, &tracked(t), heading(9.0)) + CELL_PAD_X * 2.0)
            .collect();
        let track_w: f32 = PAD * 2.0 + widths.iter().sum::<f32>() + GAP * (cells.len() as f32 - 1.0);

        let track_cy = cy + 5.0;
        let track = Rect::from_min_max(
            Pos2::new(right_x - track_w, track_cy - TRACK_H / 2.0),
            Pos2::new(right_x, track_cy + TRACK_H / 2.0),
        );
        lcd_panel(painter, track, pal, 4);

        painter.text(
            Pos2::new(track.left(), track.top() - 3.0),
            Align2::LEFT_BOTTOM,
            &tracked(micro),
            mono(7.0),
            pal.sub,
        );

        let cell_h = TRACK_H - PAD * 2.0;
        let mut x = track.left() + PAD;
        let mut clicks = Vec::with_capacity(cells.len());
        for (i, ((label, active), w)) in cells.iter().zip(widths.iter()).enumerate() {
            let cell = Rect::from_min_size(Pos2::new(x, track.top() + PAD), Vec2::new(*w, cell_h));
            let resp = key_cell(ui, painter, pal, cell, label, *active, ui.id().with((id_src, i)));
            clicks.push(resp.clicked());
            x += w + GAP;
        }
        (track.left(), clicks)
    }
}

/// One recessed LCD clock chip flush to `right_x`; returns its left edge.
fn lcd_clock(painter: &egui::Painter, pal: &Palette, right_x: f32, cy: f32, label: &str, value: &str) -> f32 {
    const READOUT_W: f32 = 79.0;
    const PAD_X: f32 = 12.0;
    const GAP: f32 = 8.0;
    const H: f32 = 26.0;

    let label_t = tracked(label);
    let label_w = measure(painter, &label_t, mono(8.0));
    let chip_w = PAD_X + label_w + GAP + READOUT_W + PAD_X;
    let chip = Rect::from_min_max(
        Pos2::new(right_x - chip_w, cy - H / 2.0),
        Pos2::new(right_x, cy + H / 2.0),
    );
    lcd_panel(painter, chip, pal, 3);

    let lx = chip.left() + PAD_X;
    painter.text(
        Pos2::new(lx, cy),
        Align2::LEFT_CENTER,
        &label_t,
        mono(8.0),
        pal.lcd_text.gamma_multiply(0.6),
    );
    let cell = Rect::from_min_max(
        Pos2::new(lx + label_w + GAP, chip.top()),
        Pos2::new(lx + label_w + GAP + READOUT_W, chip.bottom()),
    );
    // faint glow under the readout
    painter.text(cell.center(), Align2::CENTER_CENTER, value, heading_bold(16.0), pal.accent.gamma_multiply(0.18));
    painter.text(cell.center(), Align2::CENTER_CENTER, value, heading_bold(16.0), pal.lcd_text);
    chip.left()
}

// =====================================================================
// Headless screenshot driver (MARTIAN_SHOT=path)
// =====================================================================

impl App {
    fn run_screenshot(&mut self, ctx: &egui::Context) {
        let Some(path) = self.shot_path.clone() else {
            return;
        };
        self.frame += 1;
        ctx.request_repaint();
        if self.frame == 4 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        let shot = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = shot {
            let [w, h] = image.size;
            if let Some(buf) = image::RgbaImage::from_raw(w as u32, h as u32, image.as_raw().to_vec()) {
                let _ = buf.save(&path);
                eprintln!("saved screenshot: {path} ({w}x{h})");
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
