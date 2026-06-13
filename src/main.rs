//! "Martian Hybrid" theme — egui feasibility spike.
//!
//! A small eframe app that proves out the instrument-panel *chrome* (brushed-metal
//! chassis, recessed panels, flush corner brackets, spine headers, flat toggles)
//! with panels laid out by egui_tiles, flipping between a light and dark palette at
//! runtime. Panel contents are throwaway. See FEASIBILITY.md for the verdict.

mod theme;

use eframe::egui;
use egui::{
    Align2, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Mesh, Pos2, Rect, Shape,
    Stroke, StrokeKind, TextureHandle, TextureOptions, Vec2,
};
use egui_tiles::{Behavior, Tile, TileId, Tiles, Tree, UiResponse};
use theme::*;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([900.0, 560.0])
            .with_title("Martian Hybrid — egui feasibility spike"),
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

    // Headings/legends -> a dedicated "heading" family (Chakra Petch).
    fonts.families.insert(
        FontFamily::Name("heading".into()),
        vec!["chakra_bold".into(), "chakra".into()],
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
    v.selection.bg_fill = pal.accent.gamma_multiply(0.5);
    v.selection.stroke = Stroke::new(1.0, pal.accent);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, pal.body);
    v.widgets.inactive.bg_fill = pal.face_top;
    v.widgets.hovered.bg_fill = pal.face_top;
    ctx.set_visuals(v);
}

// ---------------------------------------------------------------------------
// Brushed-metal texture: a 2px-wide vertical-stripe tile, repeated.
// ---------------------------------------------------------------------------

fn make_brushed(ctx: &egui::Context, pal: &Palette) -> TextureHandle {
    // One light column, one dark column => 2px stripe period when tiled.
    let img = egui::ColorImage::new([2, 1], vec![pal.stripe_light, pal.stripe_dark]);
    ctx.load_texture("brushed", img, TextureOptions::NEAREST_REPEAT)
}

/// Paint the chassis: vertical face gradient, then the translucent brushed
/// stripes tiled over it. Used behind the tiles and under the header strip.
fn paint_chassis(painter: &egui::Painter, rect: Rect, pal: &Palette, brushed: &TextureHandle) {
    vertical_gradient(painter, rect, pal.face_top, pal.face_bottom);

    let mut mesh = Mesh::with_texture(brushed.id());
    // Repeat the 2px texture across the width; constant down the height.
    let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(rect.width() / 2.0, 1.0));
    mesh.add_rect_with_uv(rect, uv, Color32::WHITE);
    painter.add(Shape::mesh(mesh));
}

// ---------------------------------------------------------------------------
// Panes
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum PaneKind {
    Waterfall,
    Log,
    Contacts,
}

struct Pane {
    title: String,
    sub: String,
    stat: String,
    kind: PaneKind,
}

impl Pane {
    fn new(title: &str, sub: &str, stat: &str, kind: PaneKind) -> Self {
        Self {
            title: title.into(),
            sub: sub.into(),
            stat: stat.into(),
            kind,
        }
    }
}

/// Per-frame view passed to the behavior: the active palette + brushed texture,
/// plus toggle state we want to mutate from a pane footer.
struct Tactical<'a> {
    pal: &'a Palette,
    toggles: &'a mut [bool; 4],
}

impl<'a> Behavior<Pane> for Tactical<'a> {
    fn pane_ui(&mut self, ui: &mut egui::Ui, _id: TileId, pane: &mut Pane) -> UiResponse {
        let rect = ui.max_rect();
        let painter = ui.painter_at(rect);
        let pal = self.pal;

        // The header row sits directly on the chassis (already painted behind the
        // tree). Below it is the recessed screen.
        let header = Rect::from_min_size(rect.min, Vec2::new(rect.width(), PANE_HEADER_H));
        self.pane_header(&painter, header, pane);

        // Reserve a footer band only on the Contacts pane.
        let footer_h = if pane.kind == PaneKind::Contacts {
            FOOTER_H
        } else {
            0.0
        };

        let screen = Rect::from_min_max(
            Pos2::new(rect.left(), header.bottom() + 2.0),
            Pos2::new(rect.right(), rect.bottom() - footer_h),
        );
        recessed_screen(&painter, screen, pal);

        // Throwaway content clipped to inside the screen.
        let inner = screen.shrink(10.0);
        let mut content_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(inner)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        content_ui.set_clip_rect(screen.shrink(2.0));
        match pane.kind {
            PaneKind::Waterfall => self.waterfall_body(&mut content_ui, screen),
            PaneKind::Log => self.log_body(&mut content_ui),
            PaneKind::Contacts => self.contacts_body(&painter, screen),
        }

        if footer_h > 0.0 {
            let footer =
                Rect::from_min_max(Pos2::new(rect.left(), rect.bottom() - footer_h), rect.max);
            self.footer(ui, &painter, footer);
        }

        UiResponse::None
    }

    fn tab_title_for_pane(&mut self, pane: &Pane) -> egui::WidgetText {
        pane.title.clone().into()
    }

    // ---- chrome suppression: flatten everything egui_tiles would draw ----

    fn gap_width(&self, _style: &egui::Style) -> f32 {
        GAP_WIDTH // grooves: chassis shows through between panes
    }

    fn simplification_options(&self) -> egui_tiles::SimplificationOptions {
        // Keep our split tree intact; never auto-add tab bars to panes.
        egui_tiles::SimplificationOptions {
            all_panes_must_have_tabs: false,
            ..Default::default()
        }
    }

    fn resize_stroke(&self, _style: &egui::Style, state: egui_tiles::ResizeState) -> Stroke {
        // Resize handles live in the grooves. Idle => invisible (let the groove
        // read as metal); hovering/dragging => a thin accent line.
        match state {
            egui_tiles::ResizeState::Idle => Stroke::NONE,
            egui_tiles::ResizeState::Hovering => {
                Stroke::new(1.0, self.pal.accent.gamma_multiply(0.5))
            }
            egui_tiles::ResizeState::Dragging => Stroke::new(2.0, self.pal.accent),
        }
    }
}

impl<'a> Tactical<'a> {
    fn pane_header(&self, painter: &egui::Painter, rect: Rect, pane: &Pane) {
        let pal = self.pal;
        let cy = rect.center().y;
        let x0 = rect.left() + 14.0;
        // spine bar
        let after = spine(painter, Pos2::new(x0, cy), pal.accent);
        // legend
        let tx = after + 8.0;
        let title = tracked(&pane.title.to_uppercase());
        engraved_text(
            painter,
            Pos2::new(tx, cy),
            &title,
            heading(11.0),
            pal.legend,
            shadow(pal),
            Align2::LEFT_CENTER,
        );
        // sub-label, offset past the title
        let sub_x = tx + measure(painter, &title, heading(11.0)) + 12.0;
        painter.text(
            Pos2::new(sub_x, cy),
            Align2::LEFT_CENTER,
            &pane.sub,
            mono(9.0),
            pal.sub,
        );
        // right-aligned stat
        painter.text(
            Pos2::new(rect.right() - 4.0, cy),
            Align2::RIGHT_CENTER,
            &pane.stat,
            heading(9.0),
            pal.legend,
        );
    }

    fn waterfall_body(&self, ui: &mut egui::Ui, screen: Rect) {
        let pal = self.pal;
        let painter = ui.painter();
        // Fake "decode" rows on the right rail, throwaway data.
        let calls = [
            ("OH8X", "−08"),
            ("JA1NUT", "−15"),
            ("K1ABC", "−02"),
            ("DL3XYZ", "−19"),
            ("VK3WE", "−21"),
            ("W7GH", "−11"),
            ("EA7KW", "−17"),
            ("N5JR", "−05"),
            ("PY2OG", "−23"),
            ("G4ABC", "−13"),
            ("VE3EN", "−09"),
            ("ZL2AB", "−24"),
        ];
        let rail_x = screen.right() - 118.0;
        let mut y = screen.top() + 14.0;
        for (call, snr) in calls {
            if y > screen.bottom() - 12.0 {
                break;
            }
            painter.circle_filled(Pos2::new(rail_x, y), 2.5, pal.accent);
            painter.text(
                Pos2::new(rail_x + 10.0, y),
                Align2::LEFT_CENTER,
                call,
                heading(10.0),
                pal.body,
            );
            painter.text(
                Pos2::new(screen.right() - 12.0, y),
                Align2::RIGHT_CENTER,
                snr,
                mono(9.0),
                pal.dim,
            );
            y += 19.0;
        }
        // "NOW" marker line on the left of the rail.
        painter.line_segment(
            [
                Pos2::new(rail_x - 8.0, screen.top() + 6.0),
                Pos2::new(rail_x - 8.0, screen.bottom() - 6.0),
            ],
            Stroke::new(2.0, pal.accent.gamma_multiply(0.85)),
        );
        painter.text(
            Pos2::new(screen.left() + 12.0, screen.top() + 10.0),
            Align2::LEFT_CENTER,
            "0–3000 Hz",
            mono(8.5),
            pal.dim,
        );
    }

    fn log_body(&self, ui: &mut egui::Ui) {
        let pal = self.pal;
        // A couple of fake table rows via egui_extras::TableBuilder.
        use egui_extras::{Column, TableBuilder};
        let rows = [
            ("2358", "W7GH", "CN94", "−11", "−09"),
            ("2355", "JA1NUT", "PM95", "−15", "−13"),
            ("2351", "G4ABC", "IO91", "−13", "−07"),
            ("2347", "VE3EN", "FN25", "−09", "−02"),
            ("2342", "EA7KW", "IM67", "−17", "−19"),
            ("2338", "N5JR", "EM12", "−05", "−08"),
            ("2331", "OH8X", "KP24", "−08", "−14"),
        ];
        TableBuilder::new(ui)
            .striped(false)
            .column(Column::exact(44.0))
            .column(Column::remainder())
            .column(Column::exact(48.0))
            .column(Column::exact(36.0))
            .column(Column::exact(36.0))
            .header(16.0, |mut h| {
                for label in ["UTC", "CALL", "GRID", "SNT", "RCV"] {
                    h.col(|ui| {
                        ui.label(egui::RichText::new(label).font(mono(8.0)).color(pal.dim));
                    });
                }
            })
            .body(|mut body| {
                for (utc, call, grid, snt, rcv) in rows {
                    body.row(20.0, |mut row| {
                        row.col(|ui| {
                            ui.label(egui::RichText::new(utc).font(mono(10.0)).color(pal.dim));
                        });
                        row.col(|ui| {
                            ui.label(egui::RichText::new(call).font(heading(10.0)).color(pal.body));
                        });
                        row.col(|ui| {
                            ui.label(egui::RichText::new(grid).font(mono(10.0)).color(pal.dim));
                        });
                        row.col(|ui| {
                            ui.label(egui::RichText::new(snt).font(mono(10.0)).color(pal.body));
                        });
                        row.col(|ui| {
                            ui.label(egui::RichText::new(rcv).font(mono(10.0)).color(pal.accent));
                        });
                    });
                }
            });
    }

    fn contacts_body(&self, painter: &egui::Painter, screen: Rect) {
        let pal = self.pal;
        // Azimuthal radar: concentric rings + spokes + a few contact dots.
        let size = screen.height().min(screen.width()) - 24.0;
        let c = screen.center();
        let r = size * 0.5;
        let dim = Stroke::new(0.6, pal.dim);
        for k in 1..=4 {
            let rr = r * k as f32 / 4.0;
            let stroke = if k == 4 {
                Stroke::new(0.9, pal.accent.gamma_multiply(0.6))
            } else {
                dim
            };
            painter.circle_stroke(c, rr, stroke);
        }
        painter.line_segment([Pos2::new(c.x, c.y - r), Pos2::new(c.x, c.y + r)], dim);
        painter.line_segment([Pos2::new(c.x - r, c.y), Pos2::new(c.x + r, c.y)], dim);
        // Contact spots (azimuth °, range 0..1).
        let spots = [
            (8.0, 0.54),
            (322.0, 0.59),
            (30.0, 0.62),
            (250.0, 0.98),
            (46.0, 0.66),
            (112.0, 0.22),
            (132.0, 0.85),
            (25.0, 0.58),
            (72.0, 0.26),
            (232.0, 0.88),
            (158.0, 0.12),
            (352.0, 0.65),
        ];
        for (az, range) in spots {
            let a = az * std::f32::consts::PI / 180.0;
            let p = Pos2::new(c.x + r * range * a.sin(), c.y - r * range * a.cos());
            painter.line_segment([c, p], Stroke::new(0.4, pal.accent.gamma_multiply(0.32)));
            painter.circle_filled(p, 2.2, pal.accent);
        }
        painter.circle_stroke(c, 2.6, Stroke::new(1.2, pal.accent));
    }

    /// Flat tactical footer: 10px square toggles (solid = on, hollow = off) plus
    /// a small accent bar-graph. No screws, no glossy switches.
    fn footer(&mut self, ui: &mut egui::Ui, painter: &egui::Painter, rect: Rect) {
        let pal = self.pal;
        let cy = rect.center().y;
        let labels = ["DX ONLY", "CQ", "ALERT", "LOG"];
        let mut x = rect.left() + 14.0;
        for i in 0..4 {
            let sq =
                Rect::from_center_size(Pos2::new(x + TOGGLE_SQ * 0.5, cy), Vec2::splat(TOGGLE_SQ));
            // Make the square clickable to actually flip state at runtime.
            let resp = ui.interact(
                sq.expand(2.0),
                ui.id().with(("toggle", i)),
                egui::Sense::click(),
            );
            if resp.clicked() {
                self.toggles[i] = !self.toggles[i];
            }
            if self.toggles[i] {
                painter.rect_filled(sq, CornerRadius::ZERO, pal.accent);
            } else {
                painter.rect_stroke(
                    sq,
                    CornerRadius::ZERO,
                    Stroke::new(TOGGLE_STROKE, pal.sub),
                    StrokeKind::Inside,
                );
            }
            let label_color = if self.toggles[i] { pal.legend } else { pal.sub };
            let tx = sq.right() + 6.0;
            let label = tracked(labels[i]);
            painter.text(
                Pos2::new(tx, cy),
                Align2::LEFT_CENTER,
                &label,
                heading(8.5),
                label_color,
            );
            x = tx + measure(painter, &label, heading(8.5)) + 18.0;
        }

        // Bar-graph on the right.
        let heights = [5.0, 8.0, 11.0, 14.0, 9.0, 5.0];
        let mut bx = rect.right() - 70.0;
        let base = cy + 7.0;
        for (j, h) in heights.iter().enumerate() {
            let on = j < 4;
            let bar = Rect::from_min_max(Pos2::new(bx, base - h), Pos2::new(bx + 3.0, base));
            let col = if on {
                pal.accent
            } else {
                pal.sub.gamma_multiply(0.45)
            };
            painter.rect_filled(bar, CornerRadius::ZERO, col);
            bx += 5.0;
        }
        painter.text(
            Pos2::new(bx + 4.0, cy),
            Align2::LEFT_CENTER,
            "SNR",
            mono(7.5),
            pal.sub,
        );
    }
}

/// Measure rendered text width (for laying out labels by hand).
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

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

struct App {
    dark: bool,
    tree: Tree<Pane>,
    brushed: Option<TextureHandle>,
    brushed_is_dark: bool,
    visuals_set_for: Option<bool>,
    toggles: [bool; 4],
    /// If set (via MARTIAN_SHOT=path), the app renders a few frames, saves a PNG
    /// to that path, and exits — used to capture the deliverable screenshots.
    shot_path: Option<String>,
    frame: u64,
}

impl App {
    fn new() -> Self {
        // MARTIAN_LIGHT starts in the silver palette; MARTIAN_SHOT enables the
        // headless screenshot driver.
        let dark = std::env::var("MARTIAN_LIGHT").is_err();
        let shot_path = std::env::var("MARTIAN_SHOT").ok();
        Self {
            dark,
            tree: build_tree(),
            brushed: None,
            brushed_is_dark: !dark,
            visuals_set_for: None,
            toggles: [true, false, false, true], // DX ONLY + LOG on, per reference
            shot_path,
            frame: 0,
        }
    }

    fn palette(&self) -> Palette {
        if self.dark {
            GRAPHITE
        } else {
            SILVER
        }
    }
}

fn build_tree() -> Tree<Pane> {
    let mut tiles = Tiles::default();
    let waterfall = tiles.insert_pane(Pane::new(
        "Waterfall",
        "0–3000 Hz · time → left",
        "RX",
        PaneKind::Waterfall,
    ));
    let log = tiles.insert_pane(Pane::new("Log Book", "last 8 · FT8", "312 QSO", PaneKind::Log));
    let contacts = tiles.insert_pane(Pane::new(
        "Contacts",
        "azimuthal · CN87",
        "12 spots",
        PaneKind::Contacts,
    ));

    let right = tiles.insert_vertical_tile(vec![log, contacts]);
    let root = tiles.insert_horizontal_tile(vec![waterfall, right]);
    // Bias the splits so the waterfall is the large left pane.
    if let Some(Tile::Container(egui_tiles::Container::Linear(lin))) = tiles.get_mut(root) {
        lin.shares.set_share(waterfall, 1.4);
        lin.shares.set_share(right, 1.0);
    }
    Tree::new("martian_tree", root, tiles)
}

impl eframe::App for App {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root.ctx().clone();

        // Work around an upstream macOS/AppKit bug: when winit tears down the
        // window's responder chain on exit, it can throw an uncaught
        // `NSRangeException` removing a Touch Bar KVO observer. Exiting the
        // process the moment a close is requested skips that teardown entirely.
        if ctx.input(|i| i.viewport().close_requested()) {
            std::process::exit(0);
        }

        let pal = self.palette();

        // (Re)build palette-dependent state on first frame / after a flip.
        if self.visuals_set_for != Some(self.dark) {
            apply_visuals(&ctx, &pal);
            self.visuals_set_for = Some(self.dark);
        }
        if self.brushed.is_none() || self.brushed_is_dark != self.dark {
            self.brushed = Some(make_brushed(&ctx, &pal));
            self.brushed_is_dark = self.dark;
        }
        let brushed = self.brushed.clone().unwrap();

        // -------- header strip --------
        egui::Panel::top("header")
            .frame(egui::Frame::NONE)
            .exact_size(HEADER_STRIP_H + 2.0)
            .show_inside(root, |ui| {
                let rect = ui.max_rect();
                let bar =
                    Rect::from_min_max(rect.min, Pos2::new(rect.right(), rect.top() + HEADER_STRIP_H));
                let painter = ui.painter_at(rect);
                paint_chassis(&painter, bar, &pal, &brushed);
                // groove under the bar
                painter.rect_filled(
                    Rect::from_min_max(
                        Pos2::new(rect.left(), bar.bottom()),
                        Pos2::new(rect.right(), bar.bottom() + 2.0),
                    ),
                    CornerRadius::ZERO,
                    pal.edge,
                );
                self.header_ui(ui, &painter, bar);
            });

        // -------- chassis + tiles --------
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(pal.face_bottom))
            .show_inside(root, |ui| {
                let rect = ui.max_rect();
                paint_chassis(&ui.painter_at(rect), rect, &pal, &brushed);
                let mut behavior = Tactical {
                    pal: &pal,
                    toggles: &mut self.toggles,
                };
                self.tree.ui(&mut behavior, ui);
            });

        // -------- headless screenshot driver --------
        if let Some(path) = self.shot_path.clone() {
            self.frame += 1;
            ctx.request_repaint(); // keep frames flowing with no input
            if self.frame == 3 {
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
                if let Some(buf) =
                    image::RgbaImage::from_raw(w as u32, h as u32, image.as_raw().to_vec())
                {
                    let _ = buf.save(&path);
                    eprintln!("saved screenshot: {path} ({w}x{h})");
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}

impl App {
    fn header_ui(&mut self, ui: &mut egui::Ui, painter: &egui::Painter, bar: Rect) {
        let pal = self.palette();
        let cy = bar.center().y;

        // left cluster: LED + DIGIMODE + FT8 ENGINE
        let mut x = bar.left() + 28.0;
        painter.circle_filled(Pos2::new(x, cy), 4.5, pal.accent);
        x += 14.0;
        let logo = tracked("DIGIMODE");
        engraved_text(
            painter,
            Pos2::new(x, cy),
            &logo,
            heading(15.0),
            pal.legend,
            shadow(&pal),
            Align2::LEFT_CENTER,
        );
        x += measure(painter, &logo, heading(15.0)) + 12.0;
        painter.text(
            Pos2::new(x, cy),
            Align2::LEFT_CENTER,
            &tracked("FT8 ENGINE"),
            mono(9.0),
            pal.sub,
        );

        // center: recessed LCD frequency window
        let lcd = Rect::from_center_size(Pos2::new(bar.center().x, cy), Vec2::new(168.0, 26.0));
        vertical_gradient(painter, lcd, pal.lcd_top, pal.lcd_bottom);
        painter.rect_stroke(
            lcd,
            corner_radius(3),
            Stroke::new(1.0, pal.edge),
            StrokeKind::Inside,
        );
        painter.text(
            Pos2::new(lcd.left() + 12.0, cy),
            Align2::LEFT_CENTER,
            "14.074",
            heading(21.0),
            pal.lcd_text,
        );
        painter.text(
            Pos2::new(lcd.right() - 10.0, cy + 1.0),
            Align2::RIGHT_CENTER,
            "MHz · OFS 1240",
            mono(8.5),
            pal.lcd_text.gamma_multiply(0.7),
        );

        // right cluster, built right-to-left so it stays flush to the edge.
        let toggle_w = 64.0;
        let tog = Rect::from_min_size(
            Pos2::new(bar.right() - 28.0 - toggle_w, cy - 11.0),
            Vec2::new(toggle_w, 22.0),
        );
        let resp = ui.interact(tog, ui.id().with("theme_toggle"), egui::Sense::click());
        if resp.clicked() {
            self.dark = !self.dark;
        }
        vertical_gradient(painter, tog, pal.lcd_top, pal.lcd_bottom);
        painter.rect_stroke(
            tog,
            corner_radius(3),
            Stroke::new(1.0, if resp.hovered() { pal.accent } else { pal.edge }),
            StrokeKind::Inside,
        );
        painter.text(
            tog.center(),
            Align2::CENTER_CENTER,
            if self.dark { "DARK" } else { "LIGHT" },
            heading(10.0),
            pal.lcd_text,
        );

        let mut rx = tog.left() - 16.0;
        painter.text(
            Pos2::new(rx, cy),
            Align2::RIGHT_CENTER,
            "RX",
            mono(9.5),
            pal.legend,
        );
        rx -= measure(painter, "RX", mono(9.5)) + 6.0;
        painter.circle_filled(Pos2::new(rx, cy), 3.5, pal.accent);
        rx -= 16.0;
        painter.text(
            Pos2::new(rx, cy),
            Align2::RIGHT_CENTER,
            "20M",
            mono(9.5),
            pal.sub,
        );
        rx -= measure(painter, "20M", mono(9.5)) + 14.0;
        painter.text(
            Pos2::new(rx, cy),
            Align2::RIGHT_CENTER,
            "2359:45 UTC",
            mono(9.5),
            pal.legend,
        );
    }
}
