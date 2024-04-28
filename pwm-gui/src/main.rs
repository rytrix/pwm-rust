use pwm_db::{db_base::DatabaseError, db_encrypted::DatabaseEncrypted};
use pwm_lib::aes_wrapper::AesResult;
struct Vault {
    db: DatabaseEncrypted,
    changed: bool,
}

impl Vault {
    pub fn new(password: &[u8]) -> Result<Self, DatabaseError> {
        let db = DatabaseEncrypted::new(password)?;
        Ok(Self { db, changed: true })
    }

    pub fn new_from_file(file: &str, password: &[u8]) -> Result<Self, DatabaseError> {
        let contents = match std::fs::read(file) {
            Ok(contents) => match AesResult::new(contents) {
                Ok(contents) => contents,
                Err(error) => return Err(DatabaseError::InputError(error.to_string())),
            },
            Err(error) => return Err(DatabaseError::InputError(error.to_string())),
        };

        let db = DatabaseEncrypted::new_deserialize_encrypted(&contents, password)?;

        Ok(Self { db, changed: false })
    }
}

// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
// #![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;

fn define_16_9(value: f32) -> [f32; 2] {
    [value, value / (16.0 / 9.0)]
}

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "PWM Vault",
        options,
        Box::new(|_cc| Box::<MyApp>::default()),
    )
}

struct MyApp {}

impl Default for MyApp {
    fn default() -> Self {
        Self {}
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        use egui_extras::{Column, TableBuilder};

        egui::CentralPanel::default().show(ctx, |ui| {
            ctx.set_pixels_per_point(1.8);
            let _text_height = egui::TextStyle::Body
                .resolve(ui.style())
                .size
                .max(ui.spacing().interact_size.y);

            ui.horizontal(|ui| {
                ui.button("Create").clicked();
                ui.button("Open").clicked();
            });

            let table = TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::auto())
                .column(Column::remainder())
                .column(Column::remainder())
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
                                ui.add_space(5.0);
                            });
                        });
                    }
                })
        });
    }
}
