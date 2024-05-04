mod password;
mod state;
mod timer;
mod vault;

use state::State;
use timer::Timer;

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use eframe::egui;
use pwm_lib::{
    aes_wrapper::AesResult,
    crypt_file::{decrypt_file, encrypt_file},
};
use vault::Vault;

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native("PWM Vault", options, Box::new(|_cc| Box::<Gui>::default()))
}

#[derive(Debug, Clone)]
enum Event {
    Error(String),

    Create,
    Open(String),
    Save(String),
    SaveAs(String),

    Encrypt(String),
    Decrypt(String),

    Get(String),
    Insert(String, AesResult),
    Remove(String),
}

struct Gui {
    scale: f32,

    state: Arc<State>,
}

impl Default for Gui {
    fn default() -> Self {
        Self {
            scale: 1.8,

            // Need some sort of password event stack.
            // Takes password then performs the task, maybe some sort of lambda
            // Event -> Requires password -> Pass lambda to password stack
            // password stack dictates password mode
            state: Arc::new(State::default()),
        }
    }
}

impl Gui {
    fn open_file_dialog(state: Arc<State>) -> Option<PathBuf> {
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

        let file = dialog.pick_file();
        if let Some(file) = &file {
            // TODO how the hell do I handle error lol
            *state.prev_file.lock().unwrap() = Some(file.display().to_string());
        }
        // eprintln!("Selected file {}", path.display().to_string());
        file
    }

    async fn file_open(state: Arc<State>) {
        if let Some(path) = Self::open_file_dialog(state) {
            println!("{}", path.display().to_string());
        }
    }

    fn crypt_prelude(state: Arc<State>) -> Option<(String, String)> {
        let file = Self::open_file_dialog(state.clone());
        if let Some(file) = file {
            let file = file.display().to_string();

            let receiver =
                State::add_password_prompt(state.clone(), format!("Enter password for {}", file));

            let password = receiver.recv().unwrap();

            return Some((file, password));
        }

        None
    }

    async fn encrypt_file(state: Arc<State>) {
        if let Some((file, password)) = Self::crypt_prelude(state.clone()) {
            match encrypt_file(file, None, password.as_bytes()) {
                Ok(()) => (),
                Err(error) => {
                    State::add_error(state, (error.to_string(), Timer::default()));
                }
            };
        }
    }

    async fn decrypt_file(state: Arc<State>) {
        if let Some((file, password)) = Self::crypt_prelude(state.clone()) {
            match decrypt_file(file, None, password.as_bytes()) {
                Ok(()) => (),
                Err(_error) => {
                    State::add_error(
                        state,
                        (
                            String::from("Failed to decrypt file, invalid password"),
                            Timer::default(),
                        ),
                    );
                }
            };
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
                    if ui.button("Create").clicked() {
                        let state = self.state.clone();
                        tokio::spawn(async {
                            State::add_error(state, (String::from("testing"), Timer::default()));
                        });
                    }
                    if ui.button("Open").clicked() {
                        tokio::spawn(Self::file_open(self.state.clone()));
                    }
                    if ui.button("Save").clicked() {
                        // if let Some(prev_file) = &self.prev_file {
                        //     self.events.push(Event::Save(prev_file.clone()));
                        // } else {
                        //     if let Some(path) = Self::open_file_dialog() {
                        //         self.events.push(Event::SaveAs(path.display().to_string()));
                        //     }
                        // }
                    }
                    if ui.button("Save As").clicked() {
                        // if let Some(path) = Self::open_file_dialog() {
                        //     self.events.push(Event::SaveAs(path.display().to_string()));
                        // }
                    }
                });

                ui.menu_button("Options", |ui| {
                    ui.add(egui::Slider::new(&mut self.scale, 1.0..=3.0).text("UI Scale"));
                });

                ui.menu_button("Encryption", |ui| {
                    if ui.button("Encrypt File").clicked() {
                        tokio::spawn(Self::encrypt_file(self.state.clone()));
                    }
                    if ui.button("Decrypt File").clicked() {
                        tokio::spawn(Self::decrypt_file(self.state.clone()));
                    }
                });
            });
            // Top Bar End

            State::display_password_prompts(self.state.clone(), ui);

            State::display_errors(self.state.clone(), ui);

            ui.collapsing("Vault", |ui| {
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
        });
    }
}
