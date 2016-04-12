extern crate xcb;

// TODO's:
// * add more consistency to error handling
// * decide on a more consistent separation between windows and clients
// * clean up code, move around files

pub mod err;
pub mod kbd;
pub mod layout;

use std::collections::HashMap;

use xcb::base as base;
use xcb::xkb as xkb;
use xcb::xproto as xproto;

use wm::layout::Layout;

// atoms we will register
static ATOM_VEC: [&'static str; 6] = [
    "WM_PROTOCOLS", "WM_DELETE_WINDOW", "WM_STATE", "WM_TAKE_FOCUS",
    "_NET_WM_WINDOW_TYPE", "_NET_WM_TAKE_FOCUS"
];

type AtomList<'a> = Vec<(xproto::Atom, &'a str)>;

// a window manager, wrapping a Connection and a root window
pub struct Wm<'a> {
    con: &'a base::Connection,  // connection to the X server
    root: xproto::Window,       // root window
    screen: layout::ScreenSize, // screen parameters
    bindings: kbd::Keybindings, // keybindings
    clients: ClientList,        // all clients
    visible_tags: Vec<Tag>,     // all visible tags
    layout: layout::Monocle,    // the layout we will use for now
    atoms: AtomList<'a>,        // registered atoms
}

impl<'a> Wm<'a> {
    // wrap a connection to initialize a window manager
    pub fn new(con: &'a base::Connection, screen_num: i32)
        -> Result<Wm<'a>, err::WmError> {
        let setup = con.get_setup();
        if let Some(screen) = setup.roots().nth(screen_num as usize) {
            match Wm::get_atoms(con, &ATOM_VEC) {
                Ok(atoms) => Ok(Wm {con: con, root: screen.root(),
                    screen: layout::ScreenSize {width: 800, height:600},
                    bindings: HashMap::new(), clients: ClientList::new(),
                    visible_tags: vec![Tag::Foo],
                    layout: layout::default_monocle(), atoms: atoms}),
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
            | xproto::EVENT_MASK_PROPERTY_CHANGE;
        match xproto::change_window_attributes(self.con, self.root,
            &[(xproto::CW_EVENT_MASK, values)]).request_check() {
            Ok(()) => Ok(()),
            Err(_) => Err(err::WmError::OtherWmRunning)
        }
    }

    // setup keybindings
    pub fn setup_bindings(&mut self,
                          keys: Vec<(kbd::KeyPress, Box<Fn() -> ()>)>) {
        // don't grab anything for now
        xproto::ungrab_key(self.con, xproto::GRAB_ANY as u8,
                           self.root, xproto::MOD_MASK_ANY as u16);
        // compile keyboard bindings
        let mut map: HashMap<kbd::KeyPress, Box<Fn() -> ()>> =
            HashMap::with_capacity(keys.len());
        for (key, callback) in keys {
            if let Some(_) = map.insert(key, callback) {
                // found a binding for a key already registered
                println!("Overwriting binding for a key!");
            } else {
                // register for the corresponding event
                xproto::grab_key(self.con, true, self.root,
                                 key.mods as u16, key.code,
                                 xproto::GRAB_MODE_ASYNC as u8,
                                 xproto::GRAB_MODE_ASYNC as u8);
            }
        }
        self.bindings = map;
    }

    // using the current layout, arrange all visible windows
    fn arrange_windows(&self) {
        let clients = self.clients.match_clients_by_tags(&self.visible_tags);
        let geometries = self.layout.arrange(clients.len(), &self.screen);
        for (client, geometry) in clients.iter().zip(geometries.iter()) {
            if let &Some(ref geom) = geometry {
                let _ = xproto::configure_window(self.con, client.window,
                    &[(xproto::CONFIG_WINDOW_X as u16, geom.x),
                      (xproto::CONFIG_WINDOW_Y as u16, geom.y),
                      (xproto::CONFIG_WINDOW_WIDTH  as u16, geom.width),
                      (xproto::CONFIG_WINDOW_HEIGHT as u16, geom.height)]);
            } else {
                self.hide_window(client.window);
            }
        }
    }

    // hide a window by moving it offscreen
    fn hide_window(&self, window: xproto::Window) {
         xproto::configure_window(self.con, window,
                                  &[(xproto::CONFIG_WINDOW_X as u16, 1200),
                                    (xproto::CONFIG_WINDOW_Y as u16, 0)]);
         if let Ok(reply) = xproto::get_input_focus(self.con).get_reply() {
             if reply.focus() == window {
                 println!("TODO: remove focus from hidden window");
             }
         }
    }

    // set the keyboard focus on a window
    fn focus_window(&self, window: xproto::Window) {
        let _ = xproto::set_input_focus(self.con,
                                        xproto::INPUT_FOCUS_POINTER_ROOT as u8,
                                        window,
                                        xproto::TIME_CURRENT_TIME
                                        ).request_check();
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

    // TODO: maybe convert this to a trait?
    // handle an event received from the X server
    fn handle(&mut self, event: base::GenericEvent) {
        match event.response_type() {
            xkb::STATE_NOTIFY =>
                self.handle_state_notify(base::cast_event(&event)),
            xproto::PROPERTY_NOTIFY =>
                self.handle_property_notify(base::cast_event(&event)),
            xproto::CLIENT_MESSAGE =>
                self.handle_client_message(base::cast_event(&event)),
            xproto::DESTROY_NOTIFY =>
                self.handle_destroy_notify(base::cast_event(&event)),
            xproto::CONFIGURE_REQUEST =>
                self.handle_configure_request(base::cast_event(&event)),
            xproto::MAP_REQUEST =>
                self.handle_map_request(base::cast_event(&event)),
            num => println!("Ignoring event: {}.", num)
        }
    }

    // look for a matching key binding upon event receival
    fn handle_state_notify(&mut self, ev: &xkb::StateNotifyEvent) {
        let key = kbd::from_key(ev);
        println!("Key pressed: {:?}", key);
        if let Some(func) = self.bindings.get(&key) { func() }
    }

    // TODO: implement
    fn handle_property_notify(&self, ev: &xproto::PropertyNotifyEvent) {
        ()
    }

    // TODO: implement
    fn handle_client_message(&self, ev: &xproto::ClientMessageEvent) {
        ()
    }

    // a window has been destroyed, remove the corresponding client
    fn handle_destroy_notify(&mut self, ev: &xproto::DestroyNotifyEvent) {
        self.clients.remove(ev.window());
        self.arrange_windows();
    }

    // TODO: implement
    fn handle_configure_request(&self, ev: &xproto::ConfigureRequestEvent) {
        ()
    }

    // a window wants to be mapped, take necessary action
    // TODO: rewrite function
    fn handle_map_request(&mut self, ev: &xproto::MapRequestEvent) {
        let window = ev.window();
        if let Some(client) = self.clients.get_client_by_window(window) {
            println!("We need to map a window again ;)");
            return; // ugly hack to reduce scope of the borrow of self.clients
        }
        if let Some(client) =
            Client::new(self, window, self.visible_tags.clone()) {
            self.clients.add(client);
            let _ = xproto::map_window(self.con, window);
            self.focus_window(window);
        } else {
            println!("Could not create a client :(");
        }
        self.arrange_windows();
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
                return atom;
            }
        }
        // we need to put the atom in question into the static array first
        panic!("Unregistered atom used: {}!", name)
    }
    
    // get a window's EWMH property (like window type and such)
    pub fn get_ewmh_property(&self, window: xproto::Window,
                             atom_name: &'a str) -> xproto::GetPropertyCookie {
        xproto::get_property(self.con, false, window,
                             self.lookup_atom(atom_name),
                             xproto::ATOM_ATOM, 0, 0xffffffff)
    }
}

// a client wrapping a window
#[derive(Debug)]
pub struct Client {
    pub window: xproto::Window, // the window (a direct child of root)
    urgent: bool,               // is the urgency hint set?
    w_type: xproto::Atom,       // client/window type
    tags: Vec<Tag>,             // all tags this client is visible on
}

impl Client {
    // setup a new client from a window manager for a specific window
    fn new(wm: &Wm, window: xproto::Window, tags: Vec<Tag>) -> Option<Client> {
        let cookie = wm.get_ewmh_property(window, "_NET_WM_WINDOW_TYPE");
        match cookie.get_reply() {
            Ok(props) => {
                let w_type = props.type_();
                Some(Client {window: window,
                    urgent: false, w_type: w_type, tags: tags})
            },
            Err(_) => {
                None
            }
        }
    }

    // is a client visible on a set of tags?
    fn has_tags(&self, tags: &[Tag]) -> bool {
        for tag in tags {
            if self.tags.contains(tag) {
                return true;
            }
        }
        false
    }
}

// a client list, managing all direct children of the root window
struct ClientList {
    clients: Vec<Client>,
}

impl ClientList {
    // initialize an empty client list
    // TODO: decide upon an optional with_capacity() call
    pub fn new() -> ClientList {
        ClientList {clients: Vec::new()}
    }

    // get a list of references of windows that are visible on a set of tags
    fn match_clients_by_tags(&self, tags: &[Tag]) -> Vec<&Client> {
        self.clients.iter().filter(|elem| elem.has_tags(tags)).collect()
    }

    // get a client that corresponds to the given window
    pub fn get_client_by_window(&self, window: xproto::Window)
        -> Option<&Client> {
        self.clients.iter().find(|client| client.window == window)
    }

    // add a new client
    pub fn add(&mut self, client: Client) {
        self.clients.push(client);
    }

    pub fn remove(&mut self, window: xproto::Window) {
        if let Some(pos) =
            self.clients.iter().position(|elem| elem.window == window) {
            self.clients.remove(pos);
        }
    }
}

// a set of (symbolic) tags - to be extended/modified
#[derive(Debug, PartialEq, Clone)]
pub enum Tag {
    Foo,
    Bar,
    Baz,
}
