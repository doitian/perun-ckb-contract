use std::{error, fmt};

#[derive(Debug)]
pub struct Error {
    details: String,
}

impl Error {
    pub fn new(msg: &str) -> Error {
        Error {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        &self.details
    }
}

impl From<&str> for Error {
    fn from(err: &str) -> Error {
        Error::new(err)
    }
}

impl From<ckb_occupied_capacity::Error> for Error {
    fn from(err: ckb_occupied_capacity::Error) -> Error {
        Error::new(&err.to_string())
    }
}
