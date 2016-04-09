extern crate xcb;

pub mod err;
pub mod kbd;
pub mod layout;

use std::collections::HashMap;
use std::collections::LinkedList;

use xcb::base as base;
use xcb::xkb as xkb;
use xcb::xproto as xproto;

// atoms we will register
static ATOM_VEC: [&'static str; 5] = [
    "WM_PROTOCOLS", "WM_DELETE_WINDOW", "WM_STATE", "WM_TAKE_FOCUS",
    "_NET_WM_WINDOW_TYPE"
];

// a window manager, wrapping a Connection and a root window
pub struct Wm<'a> {
    con: &'a base::Connection,
    root: xproto::Window,
    bindings: HashMap<kbd::KeyPress, Box<Fn() -> ()>>,
    tags: Vec<Tag<'a>>,
    visible_tags: Vec<&'a Tag<'a>>,
    layouts: Vec<Box<layout::Layout>>,
    atoms: Vec<(xproto::Atom, &'a str)>,
}

impl<'a> Wm<'a> {
    // wrap a connection to initialize a window manager
    pub fn new(con: &'a base::Connection, screen_num: i32)
        -> Result<Wm<'a>, err::WmError> {
        let setup = con.get_setup();
        if let Some(screen) = setup.roots().nth(screen_num as usize) {
            match Wm::get_atoms(con, &ATOM_VEC) {
                Ok(atoms) => Ok(Wm {con: con, root: screen.root(),
                    bindings: HashMap::new(), atoms: atoms,
                    layouts: Vec::new(), tags: Vec::new(),
                    visible_tags: Vec::new()}),
                Err(e) => Err(e)
            }
        } else {
            Err(err::WmError::CouldNotAcquireScreen)
        }
    }

    // register window manager, by requesting substructure redirects for
    // the root window and registering all events we are interested in
    pub fn register(&self) -> Result<(), err::WmError> {
        let values
            = xproto::EVENT_MASK_SUBSTRUCTURE_REDIRECT
            | xproto::EVENT_MASK_SUBSTRUCTURE_NOTIFY
            | xproto::EVENT_MASK_PROPERTY_CHANGE
            | xproto::EVENT_MASK_KEY_PRESS
            | xproto::EVENT_MASK_BUTTON_PRESS;
        match xproto::change_window_attributes(self.con, self.root,
            &[(xproto::CW_EVENT_MASK, values)]).request_check() {
            Ok(()) => Ok(()),
            Err(_) => Err(err::WmError::OtherWmRunning)
        }
    }

    // setup keybindings
    pub fn setup_bindings(
        &mut self, keys: Vec<(kbd::KeyPress, Box<Fn() -> ()>)>) {
        let mut map: HashMap<kbd::KeyPress, Box<Fn() -> ()>> =
            HashMap::with_capacity(keys.len());
        for (key, callback) in keys {
            if let Some(_) = map.insert(key, callback) {
                println!("Overwriting binding for a key!");
            }
        }
        self.bindings = map;
    }

    // look for a matching key binding upon event receival
    fn match_key(&mut self, key: kbd::KeyPress) {
        println!("Key pressed: {:?}", key);
        if let Some(func) = self.bindings.get(&key) { func() }
    }

    // main loop: wait for events, handle them
    pub fn run(&mut self) -> Result<(), err::WmError> {
        loop {
            self.con.flush();
            if let Err(_) = self.con.has_error() {
                return Err(err::WmError::ConnectionInterrupted);
            }
            match self.con.wait_for_event() {
                Some(ev) => self.handle(ev),
                None => return Err(err::WmError::IOError)
            }
        }
    }

    // handle an event received from the X server
    fn handle(&mut self, event: base::GenericEvent) {
        match event.response_type() {
            xkb::STATE_NOTIFY =>
                self.match_key(kbd::from_key(base::cast_event(&event))),
            xproto::BUTTON_PRESS =>
                self.match_key(kbd::from_button(base::cast_event(&event))),
            xproto::PROPERTY_NOTIFY => { // TODO: find out what needs to happen here
                let ev: &xproto::PropertyNotifyEvent =
                    base::cast_event(&event);
                println!("Property changed for window {}: {}",
                         ev.window(), ev.atom());
            }
            xproto::CREATE_NOTIFY => { // TODO: add a new client, rearrange windows
                let ev: &xproto::CreateNotifyEvent = base::cast_event(&event);
                let client = Client::new(&self, ev.window());
                println!("Parent {} created window {} at x:{}, y:{}",
                         ev.parent(), ev.window(), ev.x(), ev.y());
            }
            xproto::DESTROY_NOTIFY => { // TODO: remove client, rearrange windows
                let ev: &xproto::DestroyNotifyEvent = base::cast_event(&event);
                println!("Window {} destroyed", ev.window());
            }
            xproto::CONFIGURE_REQUEST => { // TODO: find out what needs to happen here
                let ev: &xproto::ConfigureRequestEvent
                    = base::cast_event(&event);
                println!("Window {} changes geometry", ev.window());
            }
            xproto::MAP_REQUEST => { // TODO: map the window
                let ev: &xproto::MapRequestEvent = base::cast_event(&event);
                println!("Client {} requests mapping", ev.window());
            }
            num => println!("Unknown event number: {}.", num)
        }
    }

    // register and get back atoms
    fn get_atoms(con: &base::Connection, names: &[&'a str])
        -> Result<Vec<(xproto::Atom, &'a str)>, err::WmError> {
        let mut cookies = Vec::with_capacity(names.len());
        let mut res: Vec<(xproto::Atom, &'a str)> =
            Vec::with_capacity(names.len());
        for name in names {
            cookies.push((xproto::intern_atom(con, false, name), name));
        }
        for (cookie, name) in cookies {
            match cookie.get_reply() {
                Ok(r) => res.push((r.atom(), name)),
                Err(_) => return Err(
                    err::WmError::CouldNotRegisterAtom(name.to_string()))
            }
        }
        Ok(res)
    }

    // get an atom by name 
    fn lookup_atom(&self, name: &str) -> xproto::Atom {
        let tuples = self.atoms.iter();
        for &(atom, n) in tuples {
            if n == name {
                println!("Atom: {}", atom);
                return atom;
            }
        }
        panic!("Unregistered atom used!")
    }
    
    // get a window's EWMH property (like window type and such)
    pub fn get_ewmh_property(&self, window: xproto::Window,
                             atom_name: &'a str) -> xproto::GetPropertyCookie {
        xproto::get_property(self.con, false, window,
                             self.lookup_atom(atom_name),
                             xproto::ATOM_ATOM, 0, 0xffffffff)
    }
}

#[derive(Debug)]
pub struct Client {
    window: xproto::Window,
    urgent: bool,
    w_type: xproto::Atom,
}

impl Client {
    // setup a new client from a window manager for a specific window
    fn new(wm: &Wm, window: xproto::Window) -> Option<Client> {
        let cookie = wm.get_ewmh_property(window, "_NET_WM_WINDOW_TYPE");
        match cookie.get_reply() {
            Ok(props) => {
                let w_type = props.type_();
                Some(Client {window: window, urgent: false, w_type: w_type})
            },
            Err(_) => {
                None
            }
        }
    }
}

struct Tag<'a> {
    name: &'a str,
    clients: LinkedList<Client>,
}
