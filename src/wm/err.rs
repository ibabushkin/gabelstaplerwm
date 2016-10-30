use std::process::exit;

use xcb::base;

/// An error encountered by the WM.
pub enum WmError {
    /// Signal handlers handling SIGCHLD's can't be established.
    CouldNotEstablishHandlers,
    /// Could not connect to X server.
    CouldNotConnect(base::ConnError),
    /// Could not acquire a screen reference from the X server.
    CouldNotAcquireScreen,
    /// An atom used by the window manager wasn't accepted by the X server.
    CouldNotRegisterAtom(String),
    /// A border color could not be allocated with the X server.
    CouldNotAllocateColors,
    /// Another window manager is running, so we can't get the
    /// necessary event mask registered with the X server.
    OtherWmRunning,
    /// The RandR version we need is not supported.
    RandRVersionMismatch,
    /// RandR initializing event receival failed.
    RandRSetupFailed,
    /// The call to `randr::get_screen_resources` failed.
    CouldNotGetScreenResources,
    /// The set of CRTCs was empty.
    BadCrtc,
    /// The connection to the X server has been interrupted.
    ConnectionInterrupted,
    /// Input/Output with the X server has issues.
    IOError,
}

impl WmError {
    /// "Handle" an error, ie. print error message and exit.
    pub fn handle(self) -> ! {
        match self {
            WmError::CouldNotEstablishHandlers =>
                error!("could not establish signal handlers"),
            WmError::CouldNotConnect(e) =>
                error!("could not connect: {:?}", e),
            WmError::CouldNotAcquireScreen =>
                error!("could not acquire screen"),
            WmError::CouldNotRegisterAtom(s) =>
                error!("could not register atom {}", s),
            WmError::CouldNotAllocateColors =>
                error!("could not allocate border colors"),
            WmError::OtherWmRunning =>
                error!("another wm is running"),
            WmError::RandRVersionMismatch =>
                error!("randr 1.2 not supported"),
            WmError::RandRSetupFailed =>
                error!("randr setup failed"),
            WmError::CouldNotGetScreenResources =>
                error!("could not get screen resources"),
            WmError::BadCrtc =>
                error!("the set of CRTCs obtained was empty"),
            WmError::ConnectionInterrupted =>
                error!("connection interrupted"),
            WmError::IOError =>
                error!("i/o error occured"),
        };
        exit(1);
    }
}

/// Output a pseudo-logger message in case said component could not be
/// initialized (hint: that shouldn't happen).
pub fn handle_logger_error() {
    println!("ERROR:main: could not initialize logger");
    exit(1);
}
