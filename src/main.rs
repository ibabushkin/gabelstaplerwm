extern crate xcb;
//extern crate xkbcommon;

use std::process::exit;

use xcb::*;

//use xkbcommon::xkb::x11 as xkb;

// an error encountered by the WM
enum WmError {
    CouldNotConnect(ConnError),
    CouldNotAcquireScreen,
    CouldNotRegisterAtom(String),
    //CouldNotSetupXkb,
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
            //WmError::CouldNotSetupXkb =>
            //    println!("Could not setup XKB"),
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

    /* setup XKB 
    pub fn setup_xkb(&self) -> Result<(), WmError> {
        // notify the X server that we want to use XKB
        let res = xkb::use_extension(self.con,
                                     xkb::MAJOR_VERSION as u16,
                                     xkb::MINOR_VERSION as u16
                                     ).get_reply();
        if let Ok(r) = res {
            if !r.supported() {
                return Err(WmError::CouldNotSetupXkb);
            }
        } else {
            return Err(WmError::CouldNotSetupXkb);
        }

        // register for keyboard events in proper fashion (see i5wm):
        // github.com/i5-wm/i5/commit/3f5a0f0024b7c77fadea6431e356c0fc060e2986
        // NOTE: I am not sure how this works, but the XCB library docs are
        // incomplete af.
        if let Err(_) = xkb::select_events(
            self.con,                                // connection
            xkb::ID_USE_CORE_KBD as xkb::DeviceSpec, // default keyboard
            xkb::EVENT_TYPE_STATE_NOTIFY as u16,     // events we want
            0,                                       // magic :/
            xkb::EVENT_TYPE_STATE_NOTIFY as u16,     // events... again
            0xff,                                    // magic :/
            0xff,                                    // magic :/
            None                                     // no details (magic)
            ).request_check() {
            Err(WmError::CouldNotSetupXkb)
        } else {
            Ok(())
        }
    }
    */

    // register window manager, by requesting substructure redirects for
    // the root window
    pub fn register(&self) -> Result<(), WmError> {
        let values
            = EVENT_MASK_SUBSTRUCTURE_REDIRECT
            | EVENT_MASK_SUBSTRUCTURE_NOTIFY
            | EVENT_MASK_PROPERTY_CHANGE
            | EVENT_MASK_KEY_PRESS
            | EVENT_MASK_BUTTON_PRESS;
        match change_window_attributes_checked(
            self.con, self.root, &[(CW_EVENT_MASK, values)]).request_check() {
            Ok(()) => Ok(()),
            Err(_) => Err(WmError::OtherWmRunning)
        }
    }

    // main loop: wait for events, handle them
    pub fn run(&self) -> Result<(), WmError> {
        loop {
            self.con.flush();
            if let Err(_) = self.con.has_error() {
                return Err(WmError::ConnectionInterrupted);
            }
            match self.con.wait_for_event() {
                Some(ev) => self.handle(ev),
                None => return Err(WmError::IOError)
            }
        }
    }

    // handle an event received from the X server
    fn handle(&self, event: GenericEvent) {
        match event.response_type() {
            xkb::STATE_NOTIFY => {
                let ev: &xkb::StateNotifyEvent = cast_event(&event);
                println!("Key pressed: type:{}, code:{}",
                         ev.xkbType(),
                         ev.keycode());
            },
            BUTTON_PRESS => {
                let ev: &ButtonPressEvent = cast_event(&event);
                println!("Button pressed: button:{}, x:{}, y:{}",
                         ev.detail(),
                         ev.root_x(),
                         ev.root_y());
            }
            num => println!("Unknown event number: {}.", num)
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
    /* setup XKB
    if let Err(e) = wm.setup_xkb() {
        e.handle();
    }
    */
    // register as a window manager and fail if another WM is running
    if let Err(e) = wm.register() {
        e.handle();
    }
    // main loop
    if let Err(e) = wm.run() {
        e.handle();
    }
}
