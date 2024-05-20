use pwm_lib::encryption::EncryptionError;

#[derive(Debug, PartialEq, Eq)]
pub enum DatabaseError {
    NotFound,
    AlreadyExists,
    FailedHash(String),
    FailedEncryption(String),
    LockError,
    InvalidPassword,
    InputError(String),
    OutputError(String),
    FailedSerialize,
    FailedDeserialize,
    InvalidCsv(String),
    IoError(String),
    CompressionError(String),
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            Self::NotFound => f.write_str("Not found"),
            Self::AlreadyExists => f.write_str("Already exists"),
            Self::FailedHash(msg) => f.write_fmt(std::format_args!("Failed hash: {}", msg)),
            Self::FailedEncryption(msg) => f.write_fmt(std::format_args!("{}", msg)),
            Self::LockError => f.write_str("Failed to get mutex lock on db"),
            Self::InvalidPassword => f.write_str("Invalid password provided"),
            Self::InputError(msg) => f.write_fmt(std::format_args!("Input error: {}", msg)),
            Self::OutputError(msg) => f.write_fmt(std::format_args!("Output error: {}", msg)),
            Self::FailedSerialize => f.write_str("Failed to serialize"),
            Self::FailedDeserialize => f.write_str("Failed to deserialize"),
            Self::InvalidCsv(msg) => f.write_fmt(std::format_args!("Csv error: {}", msg)),
            Self::IoError(msg) => f.write_fmt(std::format_args!("Io error: {}", msg)),
            Self::CompressionError(msg) => f.write_fmt(std::format_args!("Compression error: {}", msg)),
        };
    }
}

impl std::error::Error for DatabaseError {}

impl From<csv::Error> for DatabaseError {
    fn from(value: csv::Error) -> Self {
        Self::InvalidCsv(value.to_string())
    }
}

impl From<EncryptionError> for DatabaseError {
    fn from(value: EncryptionError) -> Self {
        Self::FailedEncryption(value.to_string())
    }
}

impl From<std::io::Error> for DatabaseError {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value.to_string())
    }
}
