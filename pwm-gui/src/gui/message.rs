use std::time::Duration;

use eframe::egui::Ui;

use crate::timer::Timer;

pub struct Message {
    header: Option<String>,
    message: String,
    show_timer: bool,
    timer: Timer,
}

impl Message {
    #[allow(unused)]
    pub fn new(
        header: Option<String>,
        message: String,
        show_timer: bool,
        duration: Duration,
    ) -> Message {
        Message {
            header,
            message,
            show_timer,
            timer: Timer::new(duration),
        }
    }

    pub fn new_default_duration(
        header: Option<String>,
        message: String,
        show_timer: bool,
    ) -> Message {
        Message {
            header,
            message,
            show_timer,
            timer: Timer::default(),
        }
    }

    pub fn display(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.horizontal(|ui| {
                if let Some(header) = &self.header {
                    ui.heading(header);
                }
                ui.label(self.message.as_str());
                if self.show_timer {
                    ui.label(self.timer.remaining_time().as_secs().to_string());
                }
            });
        });
    }

    pub fn is_complete(&self) -> bool {
        self.timer.is_complete()
    }
}
