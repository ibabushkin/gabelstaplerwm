use xcb::base;

pub enum WmError {
    CouldNotEstablishSignalHandlers,
    CouldNotConnect(base::ConnError),
    CouldNotAcquireScreen,
    OtherWMRunning,
    ConnectionInterrupted,
    IOError,
}

impl WmError {
    pub fn handle(self) -> ! {
        use wm::err::WmError::*;

        match self {
            CouldNotEstablishSignalHandlers => error!("could not establish signal handlers"),
            CouldNotConnect(e) => error!("could not connect: {}", e),
            CouldNotAcquireScreen => error!("could not acquire screen"),
            OtherWMRunning => error!("another wm is running"),
            ConnectionInterrupted => error!("connection interrupted"),
            IOError => error!("I/O error occured"),
        }

        ::std::process::exit(1);
    }
}
