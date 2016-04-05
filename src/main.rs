extern crate xcb;

use std::process::exit;
use xcb::*;

// an error encountered by the WM
enum WmError {
    //CouldNotConnect(ConnError),
    CouldNotAcquireScreen,
    CouldNotRegisterAtom,
    OtherWmRunning,
}

struct Wm<'a> {
    con: &'a Connection,
    //screen: Screen,
    root: Window,
}

impl<'a> Wm<'a> {
    pub fn new(con: &'a Connection, screen_num: i32)
        -> Result<Wm<'a>, WmError> {
        let setup = con.get_setup();
        let screen = match setup.roots().nth(screen_num as usize) {
            Some(s) => s,
            None => return Err(WmError::CouldNotAcquireScreen)
        };
        Ok(Wm {con: &con, root: screen.root()})
    }

    pub fn get_atoms(&self, names: Vec<&str>) -> Result<Vec<Atom>, WmError> {
        let mut cookies = Vec::with_capacity(names.len());
        let mut res = Vec::with_capacity(names.len());
        for name in names {
            cookies.push(intern_atom(&self.con, false, name));
        }
        for cookie in cookies {
            match cookie.get_reply() {
                Ok(r) => res.push(r.atom()),
                Err(_) => return Err(WmError::CouldNotRegisterAtom)
            }
        }
        Ok(res)
    }

    pub fn register(&self) -> Result<(), WmError> {
        let values
            = EVENT_MASK_SUBSTRUCTURE_REDIRECT
            | EVENT_MASK_SUBSTRUCTURE_NOTIFY
            | EVENT_MASK_PROPERTY_CHANGE
            | EVENT_MASK_BUTTON_PRESS;
        match change_window_attributes_checked(
            &self.con, self.root, &[(CW_EVENT_MASK, values)]).request_check() {
            Ok(()) => Ok(()),
            Err(_) => Err(WmError::OtherWmRunning)
        }
    }
    pub fn run(&self) {
        loop {
            self.con.flush();
            if let Err(_) = self.con.has_error() {
                panic!("Connection interrupted!");
            }
            match self.con.wait_for_event() {
                Some(ev) => println!("Event recieved"),
                None => panic!("I/O error!")
            }
        }
    }
}

fn main() {
    let (con, screen_num) = match Connection::connect(None) {
        Ok(c) => c,
        Err(_) => panic!("Could not connect")
    };
    let wm = match Wm::new(&con, screen_num) {
        Ok(w) => w,
        Err(_) => {
            println!("error.");
            exit(1);
        }
    };
    let atoms = wm.get_atoms(vec!["WM_PROTOCOLS", "WM_DELETE_WINDOWS",
                             "WM_STATE", "WM_TAKE_FOCUS"]);
    wm.register();
    wm.run();
}
