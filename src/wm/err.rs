use std::process::exit;

use xcb::base;

/// An error encountered by the WM.
pub enum WmError {
    CouldNotEstablishHandlers,
    CouldNotConnect(base::ConnError),
    CouldNotAcquireScreen,
    CouldNotRegisterAtom(String),
    OtherWmRunning,
    ConnectionInterrupted,
    IOError,
}

impl WmError {
    /// "Handle" an error, ie. print error message and exit.
    pub fn handle(self) -> ! {
        match self {
            WmError::CouldNotEstablishHandlers => {
                error!("could not establish signal handlers")
            }
            WmError::CouldNotConnect(e) => {
                error!("could not connect: {:?}", e)
            }
            WmError::CouldNotAcquireScreen => {
                error!("could not acquire screen")
            }
            WmError::CouldNotRegisterAtom(s) => {
                error!("could not register atom {}", s)
            }
            WmError::OtherWmRunning => error!("another wm is running"),
            WmError::ConnectionInterrupted => {
                error!("connection interrupted")
            }
            WmError::IOError => error!("i/o error occured"),
        };
        exit(1);
    }
}

pub fn handle_logger_error() {
    println!("ERROR:main: could not initialize logger");
    exit(1);
}
