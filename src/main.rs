extern crate xcb;

use std::process::exit;
use xcb::*;

// an error encountered by the WM
enum WmError {
    CouldNotConnect(ConnError),
    CouldNotAcquireScreen,
    CouldNotRegisterAtom(String),
    OtherWmRunning,
    ConnectionInterrupted,
    IOError
}

impl WmError {
    // handle an error, ie. print error message and exit
    pub fn handle(self) -> ! {
        match self {
            WmError::CouldNotConnect(e) =>
                println!("Could not connect: {:?}", e),
            WmError::CouldNotAcquireScreen =>
                println!("Could not acquire screen."),
            WmError::CouldNotRegisterAtom(s) =>
                println!("Could not register atom. {}", s),
            WmError::OtherWmRunning =>
                println!("Another WM is running."),
            WmError::ConnectionInterrupted =>
                println!("Connection interrupted."),
            WmError::IOError =>
                println!("IO error occured.")
        };
        exit(1);
    }
}

// a window manager, wrapping a Connection and a root window
struct Wm<'a> {
    con: &'a Connection,
    //screen: Screen,
    root: Window,
}

impl<'a> Wm<'a> {
    // wrap a connection to initialize a window manager
    pub fn new(con: &'a Connection, screen_num: i32)
        -> Result<Wm<'a>, WmError> {
        let setup = con.get_setup();
        if let Some(screen) = setup.roots().nth(screen_num as usize) {
            Ok(Wm {con: &con, root: screen.root()})
        } else {
            Err(WmError::CouldNotAcquireScreen)
        }
    }

    // register and get back atoms
    pub fn get_atoms(&self, names: Vec<&str>) -> Result<Vec<Atom>, WmError> {
        let mut cookies = Vec::with_capacity(names.len());
        let mut res = Vec::with_capacity(names.len());
        for name in names {
            cookies.push((intern_atom(self.con, false, name), name));
        }
        for (cookie, name) in cookies {
            match cookie.get_reply() {
                Ok(r) => res.push(r.atom()),
                Err(_) =>
                    return Err(WmError::CouldNotRegisterAtom(name.to_string()))
            }
        }
        Ok(res)
    }

    // register window manager, by requesting substructure redirects for
    // the root window
    pub fn register(&self) -> Result<(), WmError> {
        let values
            = EVENT_MASK_SUBSTRUCTURE_REDIRECT
            | EVENT_MASK_SUBSTRUCTURE_NOTIFY
            | EVENT_MASK_PROPERTY_CHANGE
            | EVENT_MASK_BUTTON_PRESS;
        match change_window_attributes_checked(
            self.con, self.root, &[(CW_EVENT_MASK, values)]).request_check() {
            Ok(()) => Ok(()),
            Err(_) => Err(WmError::OtherWmRunning)
        }
    }

    // main loop: wait for events, handle them (TODO)
    pub fn run(&self) -> Result<(), WmError> {
        loop {
            self.con.flush();
            if let Err(_) = self.con.has_error() {
                return Err(WmError::ConnectionInterrupted);
            }
            match self.con.wait_for_event() {
                Some(ev) => println!("Event recieved"),
                None => return Err(WmError::IOError)
            }
        }
    }
}

fn main() {
    // new connection to X server
    let (con, screen_num) = match Connection::connect(None) {
        Ok(c) => c,
        Err(e) => WmError::CouldNotConnect(e).handle()
    };
    // wm init
    let wm = match Wm::new(&con, screen_num) {
        Ok(w) => w,
        Err(e) => e.handle()
    };
    // atom setup
    let atoms = wm.get_atoms(vec!["WM_PROTOCOLS", "WM_DELETE_WINDOWS",
                             "WM_STATE", "WM_TAKE_FOCUS"]);
    // register as a window manager and fail if another WM is running
    if let Err(e) = wm.register() {
        e.handle();
    }
    // main loop
    if let Err(e) = wm.run() {
        e.handle();
    }
}
