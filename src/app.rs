//! App state, update loop, and rendering/export helpers.

use std::path::PathBuf;

use eframe::egui;
use qrcode::types::Color;
use qrcode::QrCode;

use crate::qr_types::{build, QrKind, QrParts};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EcLevel {
    L,
    M,
    Q,
    H,
}

impl EcLevel {
    pub const ALL: &'static [EcLevel] = &[EcLevel::L, EcLevel::M, EcLevel::Q, EcLevel::H];

    pub fn label(self) -> &'static str {
        match self {
            EcLevel::L => "L \u{00B7} ~7%",
            EcLevel::M => "M \u{00B7} ~15%",
            EcLevel::Q => "Q \u{00B7} ~25%",
            EcLevel::H => "H \u{00B7} ~30%",
        }
    }

    pub fn to_qrcode(self) -> qrcode::EcLevel {
        match self {
            EcLevel::L => qrcode::EcLevel::L,
            EcLevel::M => qrcode::EcLevel::M,
            EcLevel::Q => qrcode::EcLevel::Q,
            EcLevel::H => qrcode::EcLevel::H,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Png,
    Svg,
}

impl ExportFormat {
    pub const ALL: &'static [ExportFormat] = &[ExportFormat::Png, ExportFormat::Svg];

    pub fn label(self) -> &'static str {
        match self {
            ExportFormat::Png => "PNG",
            ExportFormat::Svg => "SVG",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Png => "png",
            ExportFormat::Svg => "svg",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Style {
    pub module_size: u32, // pixels per module in the rendered output
    pub quiet_zone: u32,  // modules of border around the code
    pub foreground: [u8; 4],
    pub background: [u8; 4],
}

impl Default for Style {
    fn default() -> Self {
        Self {
            module_size: 10,
            quiet_zone: 4,
            foreground: [20, 20, 24, 255],
            background: [252, 252, 250, 255],
        }
    }
}

#[derive(Clone)]
pub struct Preview {
    pub texture: egui::TextureHandle,
    pub size_px: [usize; 2],
    pub payload: String,
}

impl std::fmt::Debug for Preview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Preview")
            .field("size_px", &self.size_px)
            .field("payload", &self.payload)
            .finish_non_exhaustive()
    }
}

pub struct QrApp {
    pub kind: QrKind,
    pub parts: QrParts,
    pub ec: EcLevel,
    pub style: Style,
    pub export_format: ExportFormat,
    pub preview: Option<Preview>,
    pub status: Option<String>,
    pub last_error: Option<String>,
}

impl QrApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Sensible default visuals
        cc.egui_ctx.set_visuals(egui::Visuals::light());
        Self {
            kind: QrKind::Url,
            parts: QrParts::default(),
            ec: EcLevel::M,
            style: Style::default(),
            export_format: ExportFormat::Png,
            preview: None,
            status: Some("Pick a content type and fill in the form.".into()),
            last_error: None,
        }
    }

    /// Returns the encoded payload string for the current `kind` / `parts`,
    /// or `None` if the inputs are empty.
    pub fn payload(&self) -> Option<String> {
        build(self.kind, &self.parts)
    }

    /// Rebuilds the preview texture if any input has changed since the last
    /// render. Idempotent and cheap to call on every frame.
    pub fn regenerate_preview(&mut self, ctx: &egui::Context) {
        let Some(payload) = self.payload() else {
            self.preview = None;
            self.last_error = None;
            self.status = Some("Fill in the form to generate a QR code.".into());
            return;
        };

        if let Some(prev) = &self.preview {
            if prev.payload == payload {
                return;
            }
        }

        match QrCode::with_error_correction_level(payload.as_bytes(), self.ec.to_qrcode()) {
            Ok(code) => {
                let (rgba, w, h) = render_rgba(&code, &self.style);
                let color_image = egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba);
                let texture =
                    ctx.load_texture("qr-preview", color_image, egui::TextureOptions::NEAREST);
                self.preview = Some(Preview {
                    texture,
                    size_px: [w, h],
                    payload,
                });
                self.last_error = None;
                self.status = Some(format!(
                    "Generated \u{2014} {} \u{00D7} {} px, {} modules",
                    w,
                    h,
                    code.width()
                ));
            }
            Err(e) => {
                self.preview = None;
                self.last_error = Some(format!("QR error: {e}"));
                self.status = Some("Failed to render QR.".into());
            }
        }
    }

    /// Save the current preview (or, if none, generate + save) to disk.
    pub fn export(&mut self, path: PathBuf) {
        let Some(payload) = self.payload() else {
            self.last_error = Some("Nothing to export \u{2014} fill the form first.".into());
            return;
        };
        let code =
            match QrCode::with_error_correction_level(payload.as_bytes(), self.ec.to_qrcode()) {
                Ok(c) => c,
                Err(e) => {
                    self.last_error = Some(format!("QR error: {e}"));
                    return;
                }
            };
        let result = match self.export_format {
            ExportFormat::Png => save_png(&code, &self.style, &path),
            ExportFormat::Svg => save_svg(&code, &path),
        };
        match result {
            Ok(()) => {
                self.last_error = None;
                self.status = Some(format!("Saved to {}", path.display()));
            }
            Err(e) => {
                self.last_error = Some(format!("Save failed: {e}"));
            }
        }
    }
}

/// Render a [`QrCode`] into an RGBA8 byte buffer at the requested pixel size
/// and with the configured foreground / background colors and quiet zone.
pub fn render_rgba(code: &QrCode, style: &Style) -> (Vec<u8>, usize, usize) {
    let modules = code.width() as u32;
    let scale = style.module_size.max(1);
    let border = style.quiet_zone;
    let side_modules = modules + 2 * border;
    let side_px = (side_modules * scale) as usize;

    let mut buf = vec![0u8; side_px * side_px * 4];
    // fill background
    for px in buf.chunks_exact_mut(4) {
        px.copy_from_slice(&style.background);
    }
    // paint modules
    for my in 0..modules {
        for mx in 0..modules {
            if code[(mx as usize, my as usize)] == Color::Dark {
                let x0 = ((mx + border) * scale) as usize;
                let y0 = ((my + border) * scale) as usize;
                for dy in 0..scale as usize {
                    let row = (y0 + dy) * side_px * 4;
                    for dx in 0..scale as usize {
                        let off = row + (x0 + dx) * 4;
                        buf[off..off + 4].copy_from_slice(&style.foreground);
                    }
                }
            }
        }
    }
    (buf, side_px, side_px)
}

fn save_png(code: &QrCode, style: &Style, path: &std::path::Path) -> std::io::Result<()> {
    use image::{ImageBuffer, Rgba};
    let (rgba, w, h) = render_rgba(code, style);
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_raw(w as u32, h as u32, rgba).expect("buffer dimensions match");
    img.save(path).map_err(std::io::Error::other)
}

fn save_svg(code: &QrCode, path: &std::path::Path) -> std::io::Result<()> {
    let svg = code.render::<qrcode::render::svg::Color<'_>>().build();
    std::fs::write(path, svg.as_bytes())
}

impl eframe::App for QrApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.regenerate_preview(ctx);
        crate::ui::build_ui(self, ctx, frame);
    }
}
