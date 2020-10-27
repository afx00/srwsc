use std::fmt;
use std::error::Error;

pub struct SrwscError {
    code: ErrorCode,
    message: String,
}

impl fmt::Display for SrwscError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let err_msg = match self.code {
            ErrorCode::ErrorAck => "Failed to get ACK message",
            ErrorCode::ErrorRequest => "Failure to request",
            ErrorCode::NotExistFile => "Not exist file",
        };

        write!(f, "{}", err_msg)
    }
}

impl fmt::Debug for SrwscError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "SrwscError {{ code: {:?}, message: {} }}",
            self.code, self.message
        )
    }
}

impl Error for SrwscError {}

impl SrwscError {
    pub fn new(code: ErrorCode, message: String) -> Self {
        SrwscError {
            code: code,
            message: message,
        }
    }
}

#[derive(Debug)]
pub enum ErrorCode {
    ErrorAck,
    ErrorRequest,
    NotExistFile,
}
