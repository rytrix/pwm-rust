use pwm_db::{db_base::DatabaseError, db_encrypted::DatabaseEncrypted};
use pwm_lib::aes_wrapper::AesResult;

use iced::widget::{button, column, text};
use iced::{Alignment, Element, Sandbox, Settings};

struct Vault {
    db: DatabaseEncrypted,
    changed: bool,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    IncrementPressed,
    DecrementPressed,
}

impl Vault {
    pub fn new(password: &[u8]) -> Result<Self, DatabaseError> {
        let db = DatabaseEncrypted::new(password)?;
        Ok(Self {
            db,
            changed: true,
        })
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

        Ok(Self {
            db,
            changed: false,
        })
    }
}

struct Gui {
    value: i32,
}

impl Sandbox for Gui {
    type Message = Message;

    fn new() -> Self {
        Gui { value: 0 }
    }

    fn title(&self) -> String {
        String::from("Vault")
    }

    fn view(&self) -> Element<Message> {
        column![
            button("Increment").on_press(Message::IncrementPressed),
            text(self.value).size(50),
            button("Decrement").on_press(Message::DecrementPressed)
        ]
        .padding(20)
        .align_items(Alignment::Center)
        .into()
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::IncrementPressed => {
                self.value += 1;
            }
            Message::DecrementPressed => {
                self.value -= 1;
            }
        }
    }
}

pub fn main() -> iced::Result {
    Gui::run(Settings::default())
}
