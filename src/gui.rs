//! The window itself (built with the `gui` feature): it draws the state of `window::Window` and passes on what the person does. Everything that can be decided without a screen is
//! decided there, in code that is tested.

use crate::browse::{self, Browser};
use crate::lang::{fill, Lang, Msg};
use crate::window::{fire, Job, Key, Kind, Mode, Section, Window, MAX_SEGMENTS};
use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, Pos2, Rect, Stroke, Vec2,
};
use std::path::PathBuf;
use std::sync::Arc;

// the colours of the theme Forge, the one the other programs of the suite start with: amber on charcoal
const BACKGROUND: Color32 = Color32::from_rgb(0x0f, 0x0d, 0x0b);
const PANEL: Color32 = Color32::from_rgb(0x17, 0x13, 0x10);
const RAISED: Color32 = Color32::from_rgb(0x22, 0x1c, 0x17);
const TEXT: Color32 = Color32::from_rgb(0xf4, 0xec, 0xe0);
const DIM: Color32 = Color32::from_rgb(0x9c, 0x8d, 0x7a);
const ACCENT: Color32 = Color32::from_rgb(0xff, 0xb0, 0x00);
const OK: Color32 = Color32::from_rgb(0x9b, 0xe3, 0x4a);
const DANGER: Color32 = Color32::from_rgb(0xff, 0x54, 0x70);
/// The head moving with the laser off, and the edge of the work.
const TRAVEL: Color32 = Color32::from_rgb(0x4a, 0x55, 0x66);
const EDGE: Color32 = Color32::from_rgb(0x4d, 0xd0, 0xe1);

enum Outcome {
    Cancel,
    Pick(PathBuf),
}

/// The window that opens a file or asks for the name of the file to write.
struct Dialog {
    browser: Browser,
    /// What the file is for.
    job: Job,
    /// The name being typed when the window asks for a file to write.
    export: Option<String>,
}

struct App {
    win: Window,
    dialog: Option<Dialog>,
    fit_pending: bool,
    /// The place of the work under the pointer.
    cursor: Option<(f64, f64)>,
}

impl App {
    fn new(lang: Lang, initial: Option<PathBuf>) -> App {
        let mut win = Window::new(lang);
        let mut fit_pending = false;
        if let Some(p) = initial {
            if win.open_image(&p) {
                win.compute();
                fit_pending = true;
            }
        }
        App {
            win,
            dialog: None,
            fit_pending,
            cursor: None,
        }
    }

    fn folder(&self) -> PathBuf {
        self.win.work_folder().unwrap_or_else(browse::start_dir)
    }

    /// Opens the window of files for a job: the list of pictures to open, or the name of the file to write.
    fn ask(&mut self, job: Job) {
        let export = job.is_export().then(|| {
            self.win
                .suggested_output_for(job)
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "engraving.nc".to_string())
        });
        self.dialog = Some(Dialog {
            browser: Browser::new(self.folder(), job.extensions()),
            job,
            export,
        });
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::Panel::top("top").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("EH3D-PHOENIX-LASER")
                        .strong()
                        .size(18.0)
                        .color(ACCENT),
                );
                ui.separator();
                if ui.button(Msg::OpenImage.tr(lang)).clicked() {
                    self.ask(Job::OpenImage);
                }
                let can = self.win.can_compute();
                if ui
                    .add_enabled(can, egui::Button::new(Msg::Compute.tr(lang)))
                    .clicked()
                    && self.win.compute()
                {
                    self.fit_pending = true;
                }
                if ui
                    .add_enabled(can, egui::Button::new(Msg::ExportGcode.tr(lang)))
                    .clicked()
                {
                    self.ask(Job::ExportGcode);
                }
                if ui
                    .add_enabled(self.win.has_work(), egui::Button::new(Msg::Fit.tr(lang)))
                    .clicked()
                {
                    self.fit_pending = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::ComboBox::from_id_salt("language")
                        .selected_text(self.win.lang.name())
                        .show_ui(ui, |ui| {
                            for l in Lang::ALL {
                                ui.selectable_value(&mut self.win.lang, l, l.name());
                            }
                        });
                    ui.label(egui::RichText::new(Msg::Language.tr(lang)).color(DIM));
                });
            });
        });
    }

    fn side_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("parameters")
            .default_size(290.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(4.0);
                    self.picture_group(ui);
                    self.mode_group(ui);
                    if self.win.mode == Mode::Engrave {
                        self.dots_group(ui);
                        self.laser_group(ui);
                    } else {
                        self.calibration_group(ui);
                    }
                    self.layers_group(ui);
                });
            });
    }

    /// The title of a group of the side panel.
    fn heading(&self, msg: Msg) -> egui::RichText {
        egui::RichText::new(msg.tr(self.win.lang))
            .strong()
            .color(ACCENT)
    }

    /// The settings of one group, each with its name above a box to type in. A setting that does not matter with the choices made is left out.
    fn fields_of(&mut self, ui: &mut egui::Ui, section: Section) {
        let lang = self.win.lang;
        for i in 0..self.win.fields.len() {
            if self.win.fields[i].section != section {
                continue;
            }
            match self.win.fields[i].key {
                Key::Overscan if self.win.overscan_auto => continue,
                Key::Accel if !self.win.overscan_auto => continue,
                Key::Threshold if self.win.kind != Kind::Threshold => continue,
                _ => {}
            }
            let label = self.win.fields[i].label.tr(lang);
            let mut text = self.win.fields[i].text.clone();
            ui.label(label);
            let edit = egui::TextEdit::singleline(&mut text).desired_width(f32::INFINITY);
            if ui.add(edit).changed() {
                self.win.set_field(i, text);
            }
        }
    }

    fn picture_group(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::CollapsingHeader::new(self.heading(Msg::SectionFile))
            .default_open(true)
            .show(ui, |ui| {
                if ui.button(Msg::OpenImage.tr(lang)).clicked() {
                    self.ask(Job::OpenImage);
                }
                let shown = self
                    .win
                    .picture_name()
                    .unwrap_or_else(|| Msg::NoFile.tr(lang).to_string());
                ui.label(egui::RichText::new(shown).color(DIM));
                if let Some(info) = self.win.info_line() {
                    ui.label(info);
                }
                ui.label(
                    egui::RichText::new(Msg::NoVectors.tr(lang))
                        .color(DIM)
                        .small(),
                );
            });
    }

    fn mode_group(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::CollapsingHeader::new(self.heading(Msg::SectionMode))
            .default_open(true)
            .show(ui, |ui| {
                let mut mode = self.win.mode;
                let shown = match mode {
                    Mode::Engrave => Msg::ModeEngrave,
                    Mode::Calibrate => Msg::ModeCalibrate,
                };
                egui::ComboBox::from_id_salt("mode")
                    .selected_text(shown.tr(lang))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut mode, Mode::Engrave, Msg::ModeEngrave.tr(lang));
                        ui.selectable_value(
                            &mut mode,
                            Mode::Calibrate,
                            Msg::ModeCalibrate.tr(lang),
                        );
                    });
                if mode != self.win.mode {
                    self.win.set_mode(mode);
                }
            });
    }

    fn dots_group(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::CollapsingHeader::new(self.heading(Msg::SectionImage))
            .default_open(true)
            .show(ui, |ui| {
                self.fields_of(ui, Section::Image);
                ui.label(Msg::Dither.tr(lang));
                let mut kind = self.win.kind;
                egui::ComboBox::from_id_salt("dither")
                    .selected_text(kind.label().tr(lang))
                    .show_ui(ui, |ui| {
                        for k in Kind::ALL {
                            ui.selectable_value(&mut kind, k, k.label().tr(lang));
                        }
                    });
                if kind != self.win.kind {
                    self.win.set_kind(kind);
                }
            });
    }

    fn laser_group(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::CollapsingHeader::new(self.heading(Msg::SectionLaser))
            .default_open(true)
            .show(ui, |ui| {
                let mut auto = self.win.overscan_auto;
                if ui.checkbox(&mut auto, Msg::OverscanAuto.tr(lang)).changed() {
                    self.win.set_overscan_auto(auto);
                }
                self.fields_of(ui, Section::Laser);
            });
    }

    fn calibration_group(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new(self.heading(Msg::SectionCalibration))
            .default_open(true)
            .show(ui, |ui| {
                self.fields_of(ui, Section::Calibration);
                // the full power value is the same setting as the one of the laser
                let i = self.win.fields.iter().position(|f| f.key == Key::SMax);
                if let Some(i) = i {
                    let label = self.win.fields[i].label.tr(self.win.lang);
                    let mut text = self.win.fields[i].text.clone();
                    ui.label(label);
                    let edit = egui::TextEdit::singleline(&mut text).desired_width(f32::INFINITY);
                    if ui.add(edit).changed() {
                        self.win.set_field(i, text);
                    }
                }
            });
    }

    fn layers_group(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        ui.add_space(6.0);
        ui.checkbox(&mut self.win.show_travel, Msg::LegendTravel.tr(lang));
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                let (text, problem) = if self.win.stale {
                    (Msg::Stale.tr(lang).to_string(), true)
                } else {
                    (self.win.status.text(lang), self.win.status.is_problem())
                };
                ui.label(egui::RichText::new(text).color(if problem { DANGER } else { OK }));
                if self.win.truncated {
                    let note = fill(Msg::Truncated.tr(lang), &[&MAX_SEGMENTS.to_string()]);
                    ui.label(egui::RichText::new(note).color(DIM));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(Msg::ZoomHint.tr(lang)).color(DIM));
                    if let Some((x, y)) = self.cursor {
                        ui.separator();
                        ui.label(egui::RichText::new(self.win.cursor_text(x, y)).color(TEXT));
                    }
                });
            });
        });
    }

    fn canvas(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::CentralPanel::no_frame().show(ui, |ui| {
            let size = ui.available_size();
            let (response, painter) = ui.allocate_painter(size, egui::Sense::click_and_drag());
            let rect = response.rect;
            let (w, h) = (f64::from(rect.width()), f64::from(rect.height()));
            painter.rect_filled(rect, 0.0, BACKGROUND);
            self.cursor = None;
            if !self.win.has_work() && self.win.mode == Mode::Engrave {
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    Msg::DropHint.tr(lang),
                    egui::FontId::proportional(18.0),
                    DIM,
                );
                return;
            }
            if self.fit_pending && w > 40.0 && h > 40.0 {
                self.win.fit(w, h);
                self.fit_pending = false;
            }
            if response.double_clicked() {
                self.fit_pending = true;
            }
            if response.dragged() {
                let d = response.drag_delta();
                self.win.viewport.pan_px(f64::from(d.x), f64::from(d.y));
            }
            if let Some(p) = response.hover_pos() {
                let at = p - rect.min;
                let (sx, sy) = (f64::from(at.x), f64::from(at.y));
                let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
                let factor = (f64::from(scroll) * 0.0015).exp() * f64::from(pinch);
                if (factor - 1.0).abs() > 1e-9 {
                    self.win.viewport.zoom_at(sx, sy, w, h, factor);
                }
                self.cursor = Some(self.win.viewport.to_world(sx, sy, w, h));
            }
            self.paint(&painter, rect);
        });
    }

    fn paint(&self, painter: &egui::Painter, rect: Rect) {
        let vp = self.win.viewport;
        let (w, h) = (f64::from(rect.width()), f64::from(rect.height()));
        let at = |x: f64, y: f64| -> Pos2 {
            let (px, py) = vp.to_screen(x, y, w, h);
            Pos2::new(rect.min.x + px as f32, rect.min.y + py as f32)
        };
        let painter = painter.with_clip_rect(rect);
        // the edge of the work
        if let Some((min, max)) = self.win.bounds() {
            let corners = vec![
                at(min.0, min.1),
                at(max.0, min.1),
                at(max.0, max.1),
                at(min.0, max.1),
            ];
            painter.add(egui::Shape::closed_line(corners, Stroke::new(1.0, EDGE)));
        }
        let line_mm = self.win.done.as_ref().map_or(0.1, |d| d.line_mm);
        let burn_width = ((line_mm * vp.px_per_mm) as f32).max(1.2);
        if self.win.show_travel {
            for s in self.win.segs.iter().filter(|s| !s.burn) {
                painter.line_segment([at(s.x0, s.y0), at(s.x1, s.y1)], Stroke::new(0.8, TRAVEL));
            }
        }
        for s in self.win.segs.iter().filter(|s| s.burn) {
            let [r, g, b] = fire(s.level);
            painter.line_segment(
                [at(s.x0, s.y0), at(s.x1, s.y1)],
                Stroke::new(burn_width, Color32::from_rgb(r, g, b)),
            );
        }
        self.paint_key(&painter, rect);
    }

    /// The key of the colours, in the corner of the picture.
    fn paint_key(&self, painter: &egui::Painter, rect: Rect) {
        if self.win.segs.is_empty() {
            return;
        }
        let lang = self.win.lang;
        let mut y = rect.min.y + 12.0;
        painter.text(
            Pos2::new(rect.min.x + 12.0, y + 7.0),
            egui::Align2::LEFT_CENTER,
            Msg::LegendBurn.tr(lang),
            egui::FontId::proportional(13.0),
            TEXT,
        );
        y += 20.0;
        let steps: u32 = 32;
        let bar = 160.0 / steps as f32;
        for k in 0..steps {
            let [r, g, b] = fire(f64::from(k) / f64::from(steps - 1));
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(rect.min.x + 12.0 + bar * k as f32, y),
                    Vec2::new(bar + 0.5, 12.0),
                ),
                0.0,
                Color32::from_rgb(r, g, b),
            );
        }
        painter.text(
            Pos2::new(rect.min.x + 12.0, y + 24.0),
            egui::Align2::LEFT_CENTER,
            Msg::LegendLow.tr(lang),
            egui::FontId::proportional(12.0),
            DIM,
        );
        painter.text(
            Pos2::new(rect.min.x + 172.0, y + 24.0),
            egui::Align2::RIGHT_CENTER,
            Msg::LegendHigh.tr(lang),
            egui::FontId::proportional(12.0),
            DIM,
        );
    }

    fn dialog_window(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.dialog.take() else {
            return;
        };
        match show_dialog(ctx, &mut dialog, self.win.lang) {
            None => self.dialog = Some(dialog),
            Some(Outcome::Cancel) => {}
            Some(Outcome::Pick(path)) => {
                if self.win.run_job(dialog.job, &path) {
                    if dialog.job == Job::OpenImage {
                        // the engraving of the new picture is worked out at once
                        self.win.compute();
                        self.fit_pending = true;
                    }
                } else {
                    // the window stays open so that another file or name can be tried
                    self.dialog = Some(dialog);
                }
            }
        }
    }
}

/// Draws the window that opens a file (or asks for the name of the file to write) and says what the person chose.
fn show_dialog(ctx: &egui::Context, d: &mut Dialog, lang: Lang) -> Option<Outcome> {
    let mut outcome = None;
    let mut open = true;
    egui::Window::new(d.job.title().tr(lang))
        .open(&mut open)
        .collapsible(false)
        .default_size([620.0, 460.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button(Msg::Up.tr(lang)).clicked() {
                    d.browser.up();
                }
                let edit = egui::TextEdit::singleline(&mut d.browser.path_text)
                    .desired_width(ui.available_width());
                let response = ui.add(edit);
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    if let Some(p) = d.browser.go_typed() {
                        if d.export.is_none() {
                            outcome = Some(Outcome::Pick(p));
                        }
                    }
                }
            });
            if let Some(e) = &d.browser.error {
                ui.colored_label(DANGER, e.as_str());
            }
            if ui
                .checkbox(&mut d.browser.show_all, Msg::AllFiles.tr(lang))
                .changed()
            {
                d.browser.refresh();
            }
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    if d.browser.entries.is_empty() {
                        ui.label(egui::RichText::new(Msg::EmptyFolder.tr(lang)).color(DIM));
                    }
                    for e in d.browser.entries.clone() {
                        let label = if e.is_dir {
                            format!("{}/", e.name)
                        } else {
                            format!("{}    ({})", e.name, browse::size_text(e.size))
                        };
                        if !ui.selectable_label(false, label).clicked() {
                            continue;
                        }
                        if e.is_dir {
                            d.browser.enter(&e.name);
                        } else if d.export.is_some() {
                            d.export = Some(e.name.clone());
                        } else if let Some(p) = d.browser.enter(&e.name) {
                            outcome = Some(Outcome::Pick(p));
                        }
                    }
                });
            ui.separator();
            ui.horizontal(|ui| {
                if let Some(name) = d.export.as_mut() {
                    ui.label(Msg::FileName.tr(lang));
                    ui.add(egui::TextEdit::singleline(name).desired_width(260.0));
                    if ui.button(Msg::Export.tr(lang)).clicked() && !name.trim().is_empty() {
                        outcome = Some(Outcome::Pick(d.browser.dir.join(name.trim())));
                    }
                }
                if ui.button(Msg::Cancel.tr(lang)).clicked() {
                    outcome = Some(Outcome::Cancel);
                }
            });
        });
    if !open && outcome.is_none() {
        outcome = Some(Outcome::Cancel);
    }
    outcome
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // a picture dropped on the window is opened
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .collect()
        });
        if let Some(p) = dropped.first() {
            if self.win.open_dropped(p) {
                self.win.compute();
                self.fit_pending = true;
            }
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::O)) {
            self.ask(Job::OpenImage);
        }
        self.top_bar(ui);
        self.status_bar(ui);
        self.side_panel(ui);
        self.canvas(ui);
        self.dialog_window(&ctx);
    }
}

/// The icon of the program for the window, from the file of the brand.
fn window_icon() -> Option<egui::IconData> {
    let bytes: &[u8] = include_bytes!("../images/brand/icon-256.png");
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    buf.truncate(info.buffer_size());
    Some(egui::IconData {
        rgba: buf,
        width: info.width,
        height: info.height,
    })
}

/// The fonts of the system that draw Japanese and Chinese are added to the ones of egui, which have no such letters.
fn install_fonts(ctx: &egui::Context) {
    let candidates = [
        "C:/Windows/Fonts/msyh.ttc",
        "C:/Windows/Fonts/YuGothM.ttc",
        "C:/Windows/Fonts/meiryo.ttc",
        "C:/Windows/Fonts/simsun.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/System/Library/Fonts/PingFang.ttc",
    ];
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            let mut fonts = FontDefinitions::default();
            fonts
                .font_data
                .insert("cjk".into(), Arc::new(FontData::from_owned(bytes)));
            for family in [FontFamily::Monospace, FontFamily::Proportional] {
                fonts.families.entry(family).or_default().push("cjk".into());
            }
            ctx.set_fonts(fonts);
            return;
        }
    }
}

/// The colours of the theme Forge for the controls of egui.
fn apply_theme(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = RAISED;
    v.extreme_bg_color = Color32::from_rgb(0x14, 0x11, 0x0e);
    v.override_text_color = Some(TEXT);
    v.selection.bg_fill = Color32::from_rgba_unmultiplied(0xff, 0xb0, 0x00, 110);
    v.selection.stroke = Stroke::new(1.5, ACCENT);
    v.widgets.inactive.bg_stroke =
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(0xff, 0xb0, 0x00, 110));
    v.widgets.inactive.weak_bg_fill = RAISED;
    v.widgets.hovered.bg_stroke = Stroke::new(1.5, ACCENT);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x33, 0x28, 0x1c);
    v.widgets.active.bg_stroke = Stroke::new(2.0, ACCENT);
    v.widgets.inactive.corner_radius = 6.0.into();
    v.widgets.hovered.corner_radius = 6.0.into();
    v.widgets.active.corner_radius = 6.0.into();
    ctx.set_visuals(v);
}

/// Opens the window. A picture given here is opened at once.
pub fn run(initial: Option<PathBuf>, lang: Lang) -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1180.0, 760.0])
        .with_min_inner_size([760.0, 480.0])
        .with_title("EH3D-PHOENIX-LASER");
    if let Some(icon) = window_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "EH3D-PHOENIX-LASER",
        options,
        Box::new(move |cc| {
            install_fonts(&cc.egui_ctx);
            apply_theme(&cc.egui_ctx);
            Ok(Box::new(App::new(lang, initial)))
        }),
    )
}
