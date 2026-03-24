use std::fmt;

#[derive(Debug, Clone)]
pub struct Error {
    pub message: String,
    pub position: usize,
}

impl Error {
    pub fn new(msg: impl Into<String>, pos: usize) -> Self {
        Self { message: msg.into(), position: pos }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JSON error at {}: {}", self.position, self.message)
    }
}

impl std::error::Error for Error {}