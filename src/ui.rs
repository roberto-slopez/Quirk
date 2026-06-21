//! UI panels: type selector, dynamic form, preview, style controls.

use eframe::egui::{self, RichText};

use crate::app::{EcLevel, ExportFormat, QrApp};
use crate::qr_types::{
    CryptoKind, EventParams, GeoParams, QrKind, SmsParams, VCardParams, WifiParams, WifiSecurity,
};

pub fn build_ui(app: &mut QrApp, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    egui::TopBottomPanel::top("quirk_top").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.heading("Quirk");
            ui.label(RichText::new("\u{2014} QR code generator").weak());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.hyperlink_to(
                    "github.com/RobESco/Quirk",
                    "https://github.com/RobESco/Quirk",
                );
            });
        });
    });

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.columns(2, |cols| {
            form_column(&mut cols[0], app);
            preview_column(&mut cols[1], app, ctx, _frame);
        });
    });
}

fn form_column(ui: &mut egui::Ui, app: &mut QrApp) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.group(|ui| {
                ui.label(RichText::new("Content type").strong());
                ui.horizontal_wrapped(|ui| {
                    for kind in QrKind::ALL {
                        let label = kind.label();
                        if ui.selectable_label(app.kind == *kind, label).clicked() {
                            app.kind = *kind;
                        }
                    }
                });
            });

            ui.add_space(8.0);
            kind_form(ui, app);

            ui.add_space(12.0);
            ui.group(|ui| {
                ui.label(RichText::new("Style").strong());

                ui.horizontal(|ui| {
                    ui.label("Error correction:");
                    egui::ComboBox::from_id_salt("ec-level")
                        .selected_text(app.ec.label())
                        .show_ui(ui, |ui| {
                            for ec in EcLevel::ALL {
                                ui.selectable_value(&mut app.ec, *ec, ec.label());
                            }
                        });
                });

                ui.horizontal(|ui| {
                    ui.label("Module size:");
                    ui.add(
                        egui::DragValue::new(&mut app.style.module_size)
                            .range(1..=32)
                            .clamp_existing_to_range(true),
                    );
                    ui.label("px");
                });

                ui.horizontal(|ui| {
                    ui.label("Quiet zone:");
                    ui.add(
                        egui::DragValue::new(&mut app.style.quiet_zone)
                            .range(0..=16)
                            .clamp_existing_to_range(true),
                    );
                    ui.label("modules");
                });

                ui.horizontal(|ui| {
                    color_picker(ui, "Foreground", &mut app.style.foreground);
                    color_picker(ui, "Background", &mut app.style.background);
                });
            });
        });
}

fn preview_column(
    ui: &mut egui::Ui,
    app: &mut QrApp,
    ctx: &egui::Context,
    _frame: &mut eframe::Frame,
) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(12.0))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Preview").strong());
                ui.add_space(8.0);
                if let Some(prev) = &app.preview {
                    let avail = ui.available_size_before_wrap();
                    let side = avail.x.min(avail.y - 80.0).max(120.0);
                    let tint = egui::Color32::WHITE;
                    ui.add(
                        egui::Image::from_texture(&prev.texture)
                            .fit_to_exact_size(egui::vec2(side, side))
                            .tint(tint),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(format!(
                            "{} \u{00D7} {} px",
                            prev.size_px[0], prev.size_px[1]
                        ))
                        .weak()
                        .small(),
                    );
                } else {
                    let avail = ui.available_size_before_wrap();
                    let side = avail.x.min(avail.y - 80.0).max(120.0);
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::hover());
                    ui.painter()
                        .rect_filled(rect, 4.0, egui::Color32::from_gray(240));
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "No QR yet",
                        egui::FontId::proportional(18.0),
                        egui::Color32::from_gray(150),
                    );
                }
            });
        });

    ui.add_space(8.0);

    ui.group(|ui| {
        ui.label(RichText::new("Export").strong());
        ui.horizontal(|ui| {
            ui.label("Format:");
            egui::ComboBox::from_id_salt("export-format")
                .selected_text(app.export_format.label())
                .show_ui(ui, |ui| {
                    for f in ExportFormat::ALL {
                        ui.selectable_value(&mut app.export_format, *f, f.label());
                    }
                });
            if ui.button("Save as\u{2026}").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter(app.export_format.label(), &[app.export_format.extension()])
                    .set_file_name(format!("quirk.{}", app.export_format.extension()))
                    .save_file()
                {
                    app.export(path);
                }
            }
            if ui.button("Copy SVG").clicked() {
                if let Some(payload) = app.payload() {
                    if let Ok(code) = qrcode::QrCode::with_error_correction_level(
                        payload.as_bytes(),
                        app.ec.to_qrcode(),
                    ) {
                        let svg = code.render::<qrcode::render::svg::Color<'_>>().build();
                        ctx.copy_text(svg.clone());
                        app.last_error = None;
                        app.status = Some(format!("Copied SVG to clipboard ({} bytes)", svg.len()));
                    }
                }
            }
            if ui.button("Copy data").clicked() {
                if let Some(payload) = app.payload() {
                    ctx.copy_text(payload.clone());
                    app.last_error = None;
                    app.status = Some("Copied payload to clipboard.".into());
                }
            }
        });
    });

    ui.add_space(8.0);
    if let Some(err) = &app.last_error {
        ui.colored_label(egui::Color32::from_rgb(180, 50, 50), err);
    } else if let Some(status) = &app.status {
        ui.label(RichText::new(status).weak());
    }
}

fn color_picker(ui: &mut egui::Ui, label: &str, rgba: &mut [u8; 4]) {
    let mut color = egui::Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3]);
    ui.label(label);
    if ui.color_edit_button_srgba(&mut color).changed() {
        rgba[0] = color.r();
        rgba[1] = color.g();
        rgba[2] = color.b();
        rgba[3] = color.a();
    }
}

fn kind_form(ui: &mut egui::Ui, app: &mut QrApp) {
    ui.group(|ui| match app.kind {
        QrKind::Text | QrKind::Url => {
            let hint = if app.kind == QrKind::Url {
                "https://example.com"
            } else {
                "Any text"
            };
            labeled_multiline(ui, "Content", &mut app.parts.text, Some(hint), 4);
        }
        QrKind::Wifi => wifi_form(ui, &mut app.parts.wifi),
        QrKind::VCard => vcard_form(ui, &mut app.parts.vcard),
        QrKind::Email => email_form(ui, &mut app.parts.email),
        QrKind::Phone => {
            labeled(
                ui,
                "Phone number",
                &mut app.parts.phone.phone,
                Some("+15551234567"),
            );
        }
        QrKind::Sms => sms_form(ui, &mut app.parts.sms),
        QrKind::Geo => geo_form(ui, &mut app.parts.geo),
        QrKind::Event => event_form(ui, &mut app.parts.event),
        QrKind::Crypto => crypto_form(ui, &mut app.parts.crypto),
    });
}

fn wifi_form(ui: &mut egui::Ui, w: &mut WifiParams) {
    labeled(ui, "Network name (SSID)", &mut w.ssid, Some("MyWiFi"));
    ui.horizontal(|ui| {
        ui.label("Security:");
        egui::ComboBox::from_id_salt("wifi-sec")
            .selected_text(w.security.label())
            .show_ui(ui, |ui| {
                for s in WifiSecurity::ALL {
                    ui.selectable_value(&mut w.security, *s, s.label());
                }
            });
    });
    let pass_enabled = w.security != WifiSecurity::None;
    ui.add_enabled_ui(pass_enabled, |ui| {
        labeled(ui, "Password", &mut w.password, Some("secret"));
    });
    ui.checkbox(&mut w.hidden, "Hidden network");
}

fn vcard_form(ui: &mut egui::Ui, v: &mut VCardParams) {
    labeled(ui, "Full name *", &mut v.full_name, Some("Ada Lovelace"));
    labeled(ui, "Organization", &mut v.org, None);
    labeled(ui, "Title", &mut v.title, None);
    labeled(ui, "Phone", &mut v.phone, Some("+15551234567"));
    labeled(ui, "Email", &mut v.email, Some("ada@example.com"));
    labeled(ui, "Website", &mut v.url, None);
    labeled(ui, "Address", &mut v.address, None);
    labeled_multiline(ui, "Note", &mut v.note, None, 2);
}

fn email_form(ui: &mut egui::Ui, e: &mut crate::qr_types::EmailParams) {
    labeled(ui, "To *", &mut e.to, Some("hello@example.com"));
    labeled(ui, "Subject", &mut e.subject, None);
    labeled_multiline(ui, "Body", &mut e.body, None, 3);
}

fn sms_form(ui: &mut egui::Ui, s: &mut SmsParams) {
    labeled(ui, "Phone number *", &mut s.phone, Some("+15551234567"));
    labeled_multiline(ui, "Message", &mut s.message, None, 2);
}

fn geo_form(ui: &mut egui::Ui, g: &mut GeoParams) {
    labeled(ui, "Latitude *", &mut g.latitude, Some("37.7749"));
    labeled(ui, "Longitude *", &mut g.longitude, Some("-122.4194"));
    ui.horizontal(|ui| {
        ui.label("Altitude:");
        let mut alt_str = g.altitude.clone().unwrap_or_default();
        if ui.text_edit_singleline(&mut alt_str).changed() {
            g.altitude = if alt_str.trim().is_empty() {
                None
            } else {
                Some(alt_str)
            };
        }
    });
    labeled(ui, "Label / query", &mut g.query, None);
}

fn event_form(ui: &mut egui::Ui, e: &mut EventParams) {
    labeled(ui, "Title *", &mut e.summary, Some("Team standup"));
    labeled(ui, "Location", &mut e.location, Some("Conference room A"));
    labeled_multiline(ui, "Description", &mut e.description, None, 2);
    labeled(
        ui,
        "Start * (YYYY-MM-DDTHH:MM)",
        &mut e.start,
        Some("2026-12-31T18:00"),
    );
    labeled(ui, "End", &mut e.end, Some("2026-12-31T19:00"));
    ui.checkbox(&mut e.all_day, "All-day event");
}

fn crypto_form(ui: &mut egui::Ui, c: &mut crate::qr_types::CryptoParams) {
    ui.horizontal(|ui| {
        ui.label("Coin:");
        egui::ComboBox::from_id_salt("crypto-kind")
            .selected_text(c.kind.label())
            .show_ui(ui, |ui| {
                for k in CryptoKind::ALL {
                    ui.selectable_value(&mut c.kind, *k, k.label());
                }
            });
    });
    labeled(
        ui,
        "Address *",
        &mut c.address,
        Some("bc1qexample... or 0xexample..."),
    );
    labeled(ui, "Amount", &mut c.amount, Some("0.001"));
    labeled(ui, "Label", &mut c.label, None);
    labeled_multiline(ui, "Message", &mut c.message, None, 2);
}

fn labeled(
    ui: &mut egui::Ui,
    label: &str,
    text: &mut String,
    hint: Option<&str>,
) -> egui::Response {
    ui.horizontal(|ui| {
        ui.label(label);
        let response = ui.add(
            egui::TextEdit::singleline(text)
                .hint_text(hint.unwrap_or(""))
                .desired_width(f32::INFINITY),
        );
        response
    })
    .inner
}

fn labeled_multiline(
    ui: &mut egui::Ui,
    label: &str,
    text: &mut String,
    hint: Option<&str>,
    lines: usize,
) {
    ui.vertical(|ui| {
        ui.label(label);
        ui.add(
            egui::TextEdit::multiline(text)
                .hint_text(hint.unwrap_or(""))
                .desired_width(f32::INFINITY)
                .desired_rows(lines),
        );
    });
}
