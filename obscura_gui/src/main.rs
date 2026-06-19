mod codec;

use eframe::egui;
use rfd::FileDialog;
use std::time::Instant;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 500.0])
            .with_min_inner_size([600.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Obscura Stealth Codec",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::<ObscuraApp>::default())
        }),
    )
}

struct ObscuraApp {
    input_path: String,
    output_path: String,
    password: String,
    logs: Vec<String>,
}

impl Default for ObscuraApp {
    fn default() -> Self {
        Self {
            input_path: String::new(),
            output_path: String::new(),
            password: String::new(),
            logs: vec!["[SYSTEM] Obscura Engine initialized. Standing by.".to_string()],
        }
    }
}

impl ObscuraApp {
    fn log(&mut self, msg: &str) {
        self.logs.push(msg.to_string());
    }
    
    fn execute_codec(&mut self, mode: &str) {
        if self.input_path.is_empty() || self.output_path.is_empty() || self.password.is_empty() {
            self.log("[ERROR] All fields must be filled.");
            return;
        }

        let mut ktime = 0u64;
        for (i, b) in self.password.bytes().enumerate() {
            ktime = ktime.wrapping_add((b as u64) << (i % 8));
        }

        let core = codec::CodecCore::new(ktime);
        self.log(&format!("[SYSTEM] Executing {} path...", mode));
        let start = Instant::now();
        
        let result = if mode == "compress" {
            core.compress_file(&self.input_path, &self.output_path)
        } else {
            core.decompress_file(&self.input_path, &self.output_path)
        };

        match result {
            Ok(_) => {
                self.log(&format!("[SUCCESS] Operation {} completed in {:.2}s", mode, start.elapsed().as_secs_f32()));
            }
            Err(e) => {
                self.log(&format!("[CRITICAL] Operation {} failed: {}", mode, e));
            }
        }
    }
}

impl eframe::App for ObscuraApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut style = (*ctx.style()).clone();
        style.visuals.window_fill = egui::Color32::from_rgb(10, 15, 10);
        style.visuals.panel_fill = egui::Color32::from_rgb(10, 15, 10);
        style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(20, 25, 20);
        style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(30, 40, 30);
        style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(40, 60, 40);
        style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(50, 150, 50);
        style.visuals.selection.bg_fill = egui::Color32::from_rgb(50, 150, 50);
        ctx.set_style(style);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(egui::RichText::new("OBSCURA STEALTH CODEC").color(egui::Color32::from_rgb(0, 255, 100)).size(24.0));
            ui.add_space(20.0);

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Input File:").color(egui::Color32::LIGHT_GREEN));
                if ui.button("Browse...").clicked() {
                    if let Some(path) = FileDialog::new().pick_file() {
                        self.input_path = path.display().to_string();
                        self.log(&format!("Selected input: {}", self.input_path));
                    }
                }
                ui.text_edit_singleline(&mut self.input_path);
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Output File:").color(egui::Color32::LIGHT_GREEN));
                if ui.button("Browse...").clicked() {
                    if let Some(path) = FileDialog::new().save_file() {
                        self.output_path = path.display().to_string();
                        self.log(&format!("Selected output: {}", self.output_path));
                    }
                }
                ui.text_edit_singleline(&mut self.output_path);
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Password:").color(egui::Color32::LIGHT_GREEN));
                ui.add(egui::TextEdit::singleline(&mut self.password).password(true));
            });

            ui.add_space(30.0);

            ui.horizontal(|ui| {
                let btn_size = egui::vec2(280.0, 40.0);
                if ui.add_sized(btn_size, egui::Button::new(egui::RichText::new("COMPRESS & ENCRYPT").size(16.0).color(egui::Color32::BLACK))).clicked() {
                    self.execute_codec("compress");
                }

                if ui.add_sized(btn_size, egui::Button::new(egui::RichText::new("DECOMPRESS & DECRYPT").size(16.0).color(egui::Color32::BLACK))).clicked() {
                    self.execute_codec("decompress");
                }
            });

            ui.add_space(30.0);
            ui.separator();
            ui.label(egui::RichText::new("TERMINAL LOG").color(egui::Color32::from_rgb(0, 200, 100)));
            
            egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                for log in &self.logs {
                    let color = if log.contains("[ERROR]") || log.contains("[CRITICAL]") {
                        egui::Color32::RED
                    } else if log.contains("[SUCCESS]") {
                        egui::Color32::GREEN
                    } else {
                        egui::Color32::LIGHT_GRAY
                    };
                    ui.label(egui::RichText::new(log).color(color).family(egui::FontFamily::Monospace));
                }
            });
        });
    }
}
