use std::collections::HashMap;

use xcb::base as base;
use xcb::xkb as xkb;
use xcb::xproto as xproto;

use wm::client::*;
use wm::err::*;
use wm::kbd::*;
use wm::layout::*;

// atoms we will register
static ATOM_VEC: [&'static str; 6] = [
    "WM_PROTOCOLS", "WM_DELETE_WINDOW", "WM_STATE", "WM_TAKE_FOCUS",
    "_NET_WM_WINDOW_TYPE", "_NET_WM_TAKE_FOCUS"
];

type AtomList<'a> = Vec<(xproto::Atom, &'a str)>;

// a window manager, wrapping a Connection and a root window
pub struct Wm<'a> {
    con: &'a base::Connection, // connection to the X server
    root: xproto::Window,      // root window
    screen: ScreenSize,        // screen parameters
    border_colors: (u32, u32), // colors available for borders 
    bindings: Keybindings,     // keybindings
    clients: ClientList,       // all clients
    tag_stack: TagStack,       // all visible tags at any point in time
    atoms: AtomList<'a>,       // registered atoms
    visible_windows: Vec<xproto::Window>, // all windows currently visible
}

impl<'a> Wm<'a> {
    // wrap a connection to initialize a window manager
    pub fn new(con: &'a base::Connection, screen_num: i32)
        -> Result<Wm<'a>, WmError> {
        let setup = con.get_setup();
        if let Some(screen) = setup.roots().nth(screen_num as usize) {
            let width = screen.width_in_pixels();
            let height = screen.height_in_pixels();
            let colormap = screen.default_colormap();
            match Wm::get_atoms(con, &ATOM_VEC) {
                Ok(atoms) => Ok(Wm {con: con, root: screen.root(),
                    screen: ScreenSize {width: width, height: height},
                    border_colors: Wm::setup_colors(con, colormap),
                    bindings: HashMap::new(), clients: ClientList::new(),
                    tag_stack: TagStack::new(), atoms: atoms,
                    visible_windows: Vec::new()}),
                Err(e) => Err(e)
            }
        } else {
            Err(WmError::CouldNotAcquireScreen)
        }
    }

    // allocate colors needed for border drawing
    fn setup_colors(con: &'a base::Connection, colormap: xproto::Colormap)
        -> (u32, u32) {
        let f_cookie = xproto::alloc_color(con, colormap, 0xff, 0x00, 0x00);
        let u_cookie = xproto::alloc_color(con, colormap, 0x00, 0xff, 0x00);
        let f_pixel = match f_cookie.get_reply() {
            Ok(reply) => reply.pixel(),
            Err(_) => panic!("Could not allocate your colors!")
        };
        let u_pixel = match u_cookie.get_reply() {
            Ok(reply) => reply.pixel(),
            Err(_) => panic!("Could not allocate your colors!")
        };
        (f_pixel, u_pixel)
    }

    // register window manager by requesting substructure redirects for
    // the root window and registering all events we are interested in
    pub fn register(&self) -> Result<(), WmError> {
        let values
            = xproto::EVENT_MASK_SUBSTRUCTURE_REDIRECT
            | xproto::EVENT_MASK_SUBSTRUCTURE_NOTIFY
            //| xproto::EVENT_MASK_KEY_PRESS uncomment for all keyboard events
            | xproto::EVENT_MASK_PROPERTY_CHANGE;
        match xproto::change_window_attributes(self.con, self.root,
            &[(xproto::CW_EVENT_MASK, values)]).request_check() {
            Ok(()) => Ok(()),
            Err(_) => Err(WmError::OtherWmRunning)
        }
    }

    // setup keybindings
    pub fn setup_bindings(&mut self, keys: Vec<(KeyPress, KeyCallback)>) {
        // don't grab anything for now
        xproto::ungrab_key(self.con, xproto::GRAB_ANY as u8,
                           self.root, xproto::MOD_MASK_ANY as u16);
        // compile keyboard bindings
        let mut map: Keybindings = HashMap::with_capacity(keys.len());
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

    // set up the stack of tag(sets)
    pub fn setup_tags(&mut self, stack: TagStack) {
        self.tag_stack = stack;
    }

    // using the current layout, arrange all visible windows
    fn arrange_windows(&mut self) {
        // first, hide all visible windows ...
        for window in self.visible_windows.iter() {
            self.hide_window(*window);
        }
        // ... and reset the vector of visible windows
        self.visible_windows.clear();

        // setup current client list
        let (clients, layout) = match self.tag_stack.current() {
            Some(ref tagset) =>
                (self.clients.match_clients_by_tags(&tagset.tags),
                 &tagset.layout),
            None => return // nothing to do here
        };
        // get geometries ...
        let geometries = layout.arrange(clients.len(), &self.screen);
        for (client, geometry) in clients.iter().zip(geometries.iter()) {
            // ... and apply them if a window is to be displayed
            if let &Some(ref geom) = geometry {
                self.visible_windows.push(client.window);
                let _ = xproto::configure_window(self.con, client.window,
                    &[(xproto::CONFIG_WINDOW_X as u16, geom.x as u32),
                      (xproto::CONFIG_WINDOW_Y as u16, geom.y as u32),
                      (xproto::CONFIG_WINDOW_WIDTH  as u16, geom.width as u32),
                      (xproto::CONFIG_WINDOW_HEIGHT as u16, geom.height as u32)
                    ]);
            }
        }
    }

    // hide a window by moving it offscreen
    fn hide_window(&self, window: xproto::Window) {
        let safe_x = (self.screen.width * 2) as u32;
        xproto::configure_window(self.con, window,
            &[(xproto::CONFIG_WINDOW_X as u16, safe_x),
              (xproto::CONFIG_WINDOW_Y as u16, 0)]);
    }

    // set the keyboard focus on a window
    fn focus_window(&mut self, window: xproto::Window) {
        self.set_focused_border_color(self.border_colors.1);
        let _ = xproto::set_input_focus(
            self.con, xproto::INPUT_FOCUS_POINTER_ROOT as u8,
            window, xproto::TIME_CURRENT_TIME).request_check();
        if let Some(tagset) = self.tag_stack.current_mut() {
            tagset.focus_window(window);
        }
        self.set_focused_border_color(self.border_colors.0);
    }

    // color the borders of the currently focused window
    fn set_focused_border_color(&self, color: u32) {
        if let Some(win) = self.tag_stack.current().and_then(|t| t.focused) {
            xproto::change_window_attributes(
                self.con, win, &[(xproto::CW_BORDER_PIXEL, color)]);
        }
    }

    // focus master window if the currently focused one is gone
    // TODO: TCS
    fn revert_focus_master(&mut self, window: xproto::Window) {
        if let Some(tagset) = self.tag_stack.current_mut() {
            if let Some(&Client {window: master, ..}) =
                self.clients.match_master_by_tags(&tagset.tags) {
                if Some(window) == tagset.focused {
                    let _ = xproto::set_input_focus(
                        self.con, xproto::INPUT_FOCUS_POINTER_ROOT as u8,
                        master, xproto::TIME_CURRENT_TIME).request_check();
                    tagset.focus_window(master);
                }
            }
        }
    }

    // main loop: wait for events, handle them
    pub fn run(&mut self) -> Result<(), WmError> {
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
    // TODO: decide where we want to set border width
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
        let key = from_key(ev);
        println!("Key pressed: {:?}", key);
        if let Some(func) = self.bindings.get(&key) {
            func(&mut self.clients, &mut self.tag_stack);
        }
        self.arrange_windows();
        if let Some(win) = self.tag_stack.current().and_then(|ts| ts.focused) {
            self.focus_window(win);
        }
        println!("TagStack: {:?}", self.tag_stack);
    }

    // TODO: implement
    #[allow(unused_variables)]
    fn handle_property_notify(&self, ev: &xproto::PropertyNotifyEvent) {
        ()
    }

    // TODO: implement
    #[allow(unused_variables)]
    fn handle_client_message(&self, ev: &xproto::ClientMessageEvent) {
        ()
    }

    // a window has been destroyed, remove the corresponding client
    fn handle_destroy_notify(&mut self, ev: &xproto::DestroyNotifyEvent) {
        self.clients.remove(ev.window());
        self.revert_focus_master(ev.window());
        self.arrange_windows();
    }

    // TODO: implement
    #[allow(unused_variables)]
    fn handle_configure_request(&self, ev: &xproto::ConfigureRequestEvent) {
        ()
    }

    // a window wants to be mapped, take necessary action
    fn handle_map_request(&mut self, ev: &xproto::MapRequestEvent) {
        let window = ev.window();
        if let None = self.clients.get_client_by_window(window) {
            let tags = match self.tag_stack.current() {
                Some(tagset) => tagset.tags.clone(),
                None => vec![Tag::default()]
                    // we need to put the client somewhere
            };
            if let Some(client) = Client::new(self, window, tags) {
                let _ = xproto::map_window(self.con, window);
                self.clients.add(client);
            } else {
                println!("Could not create a client :(");
            }
            // set border width
            let _ = xproto::configure_window(
                self.con, window,
                &[(xproto::CONFIG_WINDOW_BORDER_WIDTH as u16, 1)]);
            self.focus_window(window);
            self.arrange_windows();
        }
    }

    // register and get back atoms
    fn get_atoms(con: &base::Connection, names: &[&'a str])
        -> Result<Vec<(xproto::Atom, &'a str)>, WmError> {
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
                    WmError::CouldNotRegisterAtom(name.to_string()))
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
