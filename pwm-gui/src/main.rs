mod password;
mod vault;

use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native("PWM Vault", options, Box::new(|_cc| Box::<Gui>::default()))
}

struct Gui {
    scale: f32,
    password: String,
}

impl Default for Gui {
    fn default() -> Self {
        Self {
            scale: 1.8,
            password: String::new(),
        }
    }
}

impl eframe::App for Gui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        use egui_extras::{Column, TableBuilder};

        egui::CentralPanel::default().show(ctx, |ui| {
            ctx.set_pixels_per_point(self.scale);

            let _text_height = egui::TextStyle::Body
                .resolve(ui.style())
                .size
                .max(ui.spacing().interact_size.y);

            ui.horizontal(|ui| {
                ui.menu_button("File", |ui| {
                    ui.button("Create").clicked();
                    ui.button("Open").clicked();
                    ui.button("Save").clicked();
                    ui.button("Save As").clicked();
                });

                ui.menu_button("Options", |ui| {
                    ui.add(egui::Slider::new(&mut self.scale, 1.0..=2.0).text("UI Scale"));
                });

                ui.menu_button("Encryption", |ui| {
                    ui.button("Encrypt File").clicked();
                    ui.button("Decrypt File").clicked();
                });

                ui.menu_button("Password", |ui| {
                    ui.horizontal(|ui| {
                        if ui.add(self.password()).clicked() {
                            eprintln!("Clicked");
                        }
                    });
                });
            });

            ui.separator();

            ui.collapsing("Vault", |ui| {
                let table = TableBuilder::new(ui)
                    .striped(true)
                    .resizable(true)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(Column::auto())
                    .column(Column::remainder())
                    .column(Column::auto())
                    .min_scrolled_height(0.0);

                table
                    .header(20.0, |mut header| {
                        header.col(|ui| {
                            ui.strong("Row");
                        });
                        header.col(|ui| {
                            ui.strong("Key");
                        });
                        header.col(|ui| {
                            ui.strong("Data");
                        });
                    })
                    .body(|mut body| {
                        for row_index in 0..50 {
                            let row_height = 30.0;
                            body.row(row_height, |mut row| {
                                row.col(|ui| {
                                    ui.label(row_index.to_string());
                                });
                                row.col(|ui| {
                                    ui.label("Names can be found here");
                                });
                                row.col(|ui| {
                                    // ui.checkbox(true, "Click me");
                                    ui.button("Get data").clicked();
                                    ui.add_space(8.0);
                                });
                            });
                        }
                    })
            });
        });
    }
}
