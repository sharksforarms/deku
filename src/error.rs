use std::error;
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum DekuError {
    Parse(String),
}

impl fmt::Display for DekuError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DekuError::Parse(ref err) => write!(f, "Parse error: {}", err),
        }
    }
}

impl error::Error for DekuError {
    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            _ => Some(self),
        }
    }
}

macro_rules! nom_to_layererr {
    ($typ:ty) => {
        impl From<::nom::Err<($typ, ::nom::error::ErrorKind)>> for DekuError {
            fn from(err: ::nom::Err<($typ, ::nom::error::ErrorKind)>) -> Self {
                let msg = match err {
                    ::nom::Err::Incomplete(needed) => match needed {
                        ::nom::Needed::Size(_v) => format!("incomplete data, needs more"),
                        ::nom::Needed::Unknown => format!("incomplete data"),
                    },
                    ::nom::Err::Error(e) | ::nom::Err::Failure(e) => {
                        format!("parsing error has occurred: {}", e.1.description())
                    }
                };

                DekuError::Parse(msg)
            }
        }
    };
}

nom_to_layererr!((&[u8], usize));
