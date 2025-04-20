use std::{error::Error, fmt};

#[derive(Debug, Copy, Clone)]
pub enum AppError {
    SamplerSenderError,
    CrosstermSenderError,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::SamplerSenderError => write!(f, "sampler"),
            Self::CrosstermSenderError => write!(f, "crossterm"),
        }
    }
}

impl Error for AppError {}
