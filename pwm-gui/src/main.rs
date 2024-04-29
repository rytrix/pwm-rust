mod password;
mod vault;

use std::path::PathBuf;

use eframe::egui;
use pwm_lib::{crypt_file::{self, decrypt_file, encrypt_file}, hash::HashResult};
use vault::Vault;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native("PWM Vault", options, Box::new(|_cc| Box::<Gui>::default()))
}

enum Event {
    Encrypt(String),
    Decrypt(String),
}

struct Gui {
    scale: f32,
    prev_file: Option<String>,

    password_mode: bool,
    password_buffer: String,
    password_hash: HashResult, // maybe?

    vault_name: Option<String>,
    vault: Option<Vault>,

    events: Vec<Event>,
}

impl Gui {
    fn open_file_dialogue() -> Option<PathBuf> {
        let mut dialog = rfd::FileDialog::new();

        match std::env::current_dir() {
            Ok(path) => {
                dialog = dialog.set_directory(path);
            }
            Err(error) => {
                // TODO logging
                eprintln!("Could not open current directory: {}", error.to_string());
            }
        };

        return dialog.pick_file();
        // eprintln!("Selected file {}", path.display().to_string());
    }

    fn handle_event(&mut self) {
        let event = self.events.pop();
        if let Some(event) = event {
            match event {
                Event::Encrypt(file) => {
                    encrypt_file(file, None, self.password_buffer.as_bytes()).unwrap();
                }
                Event::Decrypt(file) => {
                    decrypt_file(file, None, self.password_buffer.as_bytes()).unwrap();
                }
            }
        }
    }
}

impl Default for Gui {
    fn default() -> Self {
        Self {
            scale: 1.8,
            prev_file: Some(String::new()),

            password_mode: false,
            password_buffer: String::new(),
            password_hash: HashResult::zeroed(),

            vault_name: Some(String::from("test")),
            vault: None,

            events: Vec::new(),
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

            // Top Bar
            ui.horizontal(|ui| {
                ui.menu_button("File", |ui| {
                    ui.button("Create").clicked();
                    if ui.button("Open").clicked() {
                        if let Some(path) = Self::open_file_dialogue() {
                            self.prev_file = Some(path.display().to_string());
                            eprintln!("Selected file {}", path.display().to_string());
                            // TODO create a vault from file and password hash?
                        }
                    }
                    if ui.button("Save").clicked() {
                        // TODO save vault to prev_file
                    }
                    if ui.button("Save As").clicked() {
                        if let Some(path) = Self::open_file_dialogue() {
                            self.prev_file = Some(path.display().to_string());
                            eprintln!("Selected file {}", path.display().to_string());
                            // TODO save to prev_file
                        }
                    }
                });

                ui.menu_button("Options", |ui| {
                    ui.add(egui::Slider::new(&mut self.scale, 1.0..=2.0).text("UI Scale"));
                });

                ui.menu_button("Encryption", |ui| {
                    if ui.button("Encrypt File").clicked() {
                        if let Some(path) = Self::open_file_dialogue() {
                            self.events.push(Event::Encrypt(path.display().to_string()));
                        }
                    }
                    if ui.button("Decrypt File").clicked() {
                        if let Some(path) = Self::open_file_dialogue() {
                            self.events.push(Event::Decrypt(path.display().to_string()));
                        }
                    }
                });

                // if ui.button("Password").clicked() {
                //     self.password_mode = true;
                // }
                // ui.menu_button("Password", |ui| {
                //     ui.horizontal(|ui| {
                //         ui.add(self.password());
                //     });
                // });
            });
            // Top Bar End

            if self.events.len() != 0 {
                self.password_mode = true;
            }

            if self.password_mode {
                ui.separator();

                ui.horizontal(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Enter password");
                        ui.add(self.password());
                    });
                });
            }

            ui.separator();

            if let Some(vault_name) = &mut self.vault_name {
                ui.horizontal(|ui| {
                    ui.heading("Vault");
                    ui.text_edit_singleline(vault_name);
                });

                ui.separator();
            }

            // ui.collapsing("Vault", |ui| {
            let table = TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::auto())
                .column(Column::auto())
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
        // });
    }
}
