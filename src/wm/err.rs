use std::process::exit;

use xcb::base;

// an error encountered by the WM
pub enum WmError {
    CouldNotConnect(base::ConnError),
    CouldNotAcquireScreen,
    CouldNotRegisterAtom(String),
    OtherWmRunning,
    ConnectionInterrupted,
    IOError,
}

impl WmError {
    // handle an error, ie. print error message and exit
    pub fn handle(self) -> ! {
        match self {
            WmError::CouldNotConnect(e) => {
                println!("Could not connect: {:?}", e)
            }
            WmError::CouldNotAcquireScreen => {
                println!("Could not acquire screen.")
            }
            WmError::CouldNotRegisterAtom(s) => {
                println!("Could not register atom. {}", s)
            }
            WmError::OtherWmRunning => println!("Another WM is running."),
            WmError::ConnectionInterrupted => {
                println!("Connection interrupted.")
            }
            WmError::IOError => println!("IO error occured."),
        };
        exit(1);
    }
}
