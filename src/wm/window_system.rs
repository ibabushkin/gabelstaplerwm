use libc::c_char;

use std::collections::HashMap;
use std::ffi::CStr;
use std::mem::transmute;
use std::str;

use xcb::base;
use xcb::xkb;
use xcb::xproto;
use xcb::ffi::xcb_client_message_data_t;

use wm::client::*;
use wm::config::Tag;
use wm::err::*;
use wm::kbd::*;
use wm::layout::*;

// atoms we will register
static ATOM_VEC: [&'static str; 9] =
    ["WM_PROTOCOLS", "WM_DELETE_WINDOW", "WM_STATE",
     "WM_TAKE_FOCUS", "_NET_WM_TAKE_FOCUS", "_NET_WM_NAME", "_NET_WM_CLASS",
     "_NET_WM_WINDOW_TYPE", "_NET_WM_WINDOW_TYPE_DOCK"];

// association vector type for atoms and their names
type AtomList<'a> = Vec<(xproto::Atom, &'a str)>;

// closure type of a callback function determining client placement on
// creation, used to implement default tagsets for specific clients
pub type Matching = Box<Fn(&ClientProps) -> Option<Vec<Tag>>>;

// enumeration type used to fine-tune the behaviour after a callback
pub enum WmCommand {
    Redraw,               // redraw everything
    Focus,                // refocus window
    Kill(xproto::Window), // kill the window's process
    NoCommand,            // don't do anything
}

// configuration information used by the window manager
#[derive(Clone)]
pub struct WmConfig {
    pub f_color: (u16, u16, u16), // color of focused window's border
    pub u_color: (u16, u16, u16), // color of unfocused window's border
    pub border_width: u8,         // window border width
    pub screen: ScreenSize,       // wanted screen parameters, reset by the wm
}

// a window manager is the central instance coordinating the communication
// with the X server, as well as containing structures to manage tags
// and clients.
pub struct Wm<'a> {
    con: &'a base::Connection,  // connection to the X server
    root: xproto::Window,       // root window
    config: WmConfig,           // user defined configuration values
    screen: ScreenSize,         // screen parameters
    border_colors: (u32, u32),  // colors available for borders
    bindings: Keybindings,      // keybindings
    matching: Option<Matching>, // matching function for client placement
    clients: ClientSet,         // client set
    tag_stack: TagStack,        // tagset history
    atoms: AtomList<'a>,        // registered atoms
    visible_windows: Vec<xproto::Window>,   // all windows currently visible
    focused_window: Option<xproto::Window>, // currently focused window
    unmanaged_windows: Vec<xproto::Window>, // windows we know about,
                                            // but do not manage
}

impl<'a> Wm<'a> {
    // wrap a connection to initialize a window manager
    pub fn new(con: &'a base::Connection, screen_num: i32, config: WmConfig)
        -> Result<Wm<'a>, WmError> {
        let setup = con.get_setup();
        if let Some(screen) = setup.roots().nth(screen_num as usize) {
            let width = screen.width_in_pixels();
            let height = screen.height_in_pixels();
            let colormap = screen.default_colormap();
            let new_screen = ScreenSize::new(&config.screen, width, height);
            match Wm::get_atoms(con, &ATOM_VEC) {
                Ok(atoms) => {
                    Ok(Wm {
                        con: con,
                        root: screen.root(),
                        config: config.clone(),
                        screen: new_screen,
                        border_colors: Wm::setup_colors(con,
                                                        colormap,
                                                        config.f_color,
                                                        config.u_color),
                        bindings: HashMap::new(),
                        matching: None,
                        clients: ClientSet::new(),
                        tag_stack: TagStack::new(),
                        atoms: atoms,
                        visible_windows: Vec::new(),
                        focused_window: None,
                        unmanaged_windows: Vec::new(),
                    })
                }
                Err(e) => Err(e),
            }
        } else {
            Err(WmError::CouldNotAcquireScreen)
        }
    }

    // allocate colors needed for border drawing
    fn setup_colors(con: &'a base::Connection,
                    colormap: xproto::Colormap,
                    f_color: (u16, u16, u16),
                    u_color: (u16, u16, u16))
        -> (u32, u32) {
        let f_cookie = xproto::alloc_color(con,
                                           colormap,
                                           f_color.0,
                                           f_color.1,
                                           f_color.2);
        let u_cookie = xproto::alloc_color(con,
                                           colormap,
                                           u_color.0,
                                           u_color.1,
                                           u_color.2);
        let f_pixel = match f_cookie.get_reply() {
            Ok(reply) => reply.pixel(),
            Err(_) => panic!("Could not allocate your colors!"),
        };
        let u_pixel = match u_cookie.get_reply() {
            Ok(reply) => reply.pixel(),
            Err(_) => panic!("Could not allocate your colors!"),
        };
        (f_pixel, u_pixel)
    }

    // register window manager by requesting substructure redirects for
    // the root window and registering all events we are interested in
    pub fn register(&self) -> Result<(), WmError> {
        let values = xproto::EVENT_MASK_SUBSTRUCTURE_REDIRECT
            | xproto::EVENT_MASK_SUBSTRUCTURE_NOTIFY
            | xproto::EVENT_MASK_PROPERTY_CHANGE;
        match xproto::change_window_attributes(self.con,
                                               self.root,
                                               &[(xproto::CW_EVENT_MASK,
                                                  values)])
            .request_check() {
            Ok(()) => Ok(()),
            Err(_) => Err(WmError::OtherWmRunning),
        }
    }

    // set up keybindings
    pub fn setup_bindings(&mut self, mut keys: Vec<(KeyPress, KeyCallback)>) {
        // don't grab anything for now
        xproto::ungrab_key(self.con,
                           xproto::GRAB_ANY as u8,
                           self.root,
                           xproto::MOD_MASK_ANY as u16);
        // compile keyboard bindings
        self.bindings = HashMap::with_capacity(keys.len());
        let cookies: Vec<_> = keys
            .drain(..)
            .map(|(key, callback)| {
                if self.bindings.insert(key, callback).is_some() {
                    println!("overwriting bindings for a key!");
                }
                // register for the corresponding event
                xproto::grab_key(self.con, true, self.root,
                                 key.mods as u16, key.code,
                                 xproto::GRAB_MODE_ASYNC as u8,
                                 xproto::GRAB_MODE_ASYNC as u8)
            })
            .collect();
        for cookie in cookies {
            if cookie.request_check().is_err() {
                println!("could not grab key!");
            }
        }
    }

    // set up client matching
    pub fn setup_matching(&mut self, matching: Matching) {
        self.matching = Some(matching);
    }

    // set up the stack of tagsets
    pub fn setup_tags(&mut self, stack: TagStack) {
        self.tag_stack = stack;
    }

    // check whether we create new clients as masters or slaves
    fn new_window_as_master(&self) -> bool {
        match self.tag_stack.current() {
            Some(ref tagset) => tagset.layout.new_window_as_master(),
            _ => false,
        }
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
            Some(tagset) => (
                self.clients.get_order_or_insert(&tagset.tags),
                &tagset.layout
            ),
            None => return, // nothing to do here - no tagset on stack
        };
        // get geometries ...
        let geometries = layout.arrange(clients.1.len(), &self.screen);
        // we set geometries in serial, because otherwise window redraws are
        // rendered lazily, at least with xephyr. to avoid this condition,
        // we accept some additional waiting time, which doesn't matter much
        // anyway - redraw times aren't subject to visible latency anyway.
        // until this is fixed, the code below has to stay serial in nature
        for (client, geometry) in clients.1.iter().zip(geometries.iter()) {
            // ... and apply them if a window is to be displayed
            if let (Some(ref cl), &Some(ref geom))
                = (client.upgrade(), geometry) {
                self.visible_windows.push(cl.borrow().window);
                let cookie = xproto::configure_window(
                    self.con, cl.borrow().window,
                    &[(xproto::CONFIG_WINDOW_X as u16, geom.x as u32),
                      (xproto::CONFIG_WINDOW_Y as u16, geom.y as u32),
                      (xproto::CONFIG_WINDOW_WIDTH as u16, geom.width as u32),
                      (xproto::CONFIG_WINDOW_HEIGHT as u16, geom.height as u32)
                    ]);
                if cookie.request_check().is_err() {
                    println!("could not set window geometry");
                }
            }
        }
    }

    // hide a window by moving it offscreen
    fn hide_window(&self, window: xproto::Window) {
        let safe_x = (self.screen.width * 2) as u32;
        let cookie = xproto::configure_window(
            self.con, window, &[(xproto::CONFIG_WINDOW_X as u16, safe_x),
                                (xproto::CONFIG_WINDOW_Y as u16, 0)]);
        if cookie.request_check().is_err() {
            println!("could not move window offscreen");
        }
    }

    // destroy a window by sending a client message and killing the client the
    // hard and merciless way if that fails.
    fn destroy_window(&self, window: xproto::Window) {
        let data = [self.lookup_atom("WM_DELETE_WINDOW"), 0, 0, 0, 0].as_ptr()
            as *const xcb_client_message_data_t;
        let event: &str = unsafe {
            transmute(xproto::ClientMessageEvent::new(
                32, window, self.lookup_atom("WM_PROTOCOLS"), *data))
        };
        if xproto::send_event(self.con, false, window,
                              xproto::EVENT_MASK_NO_EVENT, event)
            .request_check().is_err() {
            if xproto::kill_client(self.con, window).request_check().is_err() {
                println!("could not kill client.");
            }
        }
    }

    // reset focus - the datastructures have been altered, we need to focus
    // the appropriate focus. if an old window is given, uncolor it's border.
    fn reset_focus(&mut self) {
        if let Some(new) = self
            .tag_stack
            .current()
            .and_then(|t| self.clients.get_focused_window(&t.tags)) {
            if self.new_window_as_master() {
               self.clients.swap_master(self.tag_stack.current().unwrap());
               self.arrange_windows();
            }
            if let Some(old_win) = self.focused_window {
                self.set_border_color(old_win, self.border_colors.1);
            }
            let cookie =
                xproto::set_input_focus(self.con,
                                        xproto::INPUT_FOCUS_POINTER_ROOT as u8,
                                        new,
                                        xproto::TIME_CURRENT_TIME);
            self.focused_window = Some(new);
            self.set_border_color(new, self.border_colors.0);
            if cookie.request_check().is_err() {
                println!("could not focus window");
            }
        }
    }

    // color the borders of a window
    fn set_border_color(&self, window: xproto::Window, color: u32) {
        let cookie =
            xproto::change_window_attributes(self.con,
                                             window,
                                             &[(xproto::CW_BORDER_PIXEL,
                                                color)]);
        if cookie.request_check().is_err() {
            println!("could not set window border color");
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
                None => return Err(WmError::IOError),
            }
        }
    }

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
            num => println!("Ignoring event: {}.", num),
        }
    }

    // look for a matching key binding upon event receival and react
    // accordingly: call a callback closure if necessary and optionally redraw
    // the screen,
    fn handle_state_notify(&mut self, ev: &xkb::StateNotifyEvent) {
        let key = from_key(ev, self.tag_stack.mode);
        println!("Key pressed: {:?}", key);
        let mut command = WmCommand::NoCommand;
        if let Some(func) = self.bindings.get(&key) {
            command = func(&mut self.clients, &mut self.tag_stack);
        }
        match command {
            WmCommand::Redraw => {
                self.arrange_windows();
                self.reset_focus();
            },
            WmCommand::Focus => self.reset_focus(),
            WmCommand::Kill(win) => self.destroy_window(win),
            WmCommand::NoCommand => (),
        };
    }

    // TODO: implement
    fn handle_property_notify(&self, _: &xproto::PropertyNotifyEvent) {
        ()
    }

    // TODO: implement
    fn handle_client_message(&self, _: &xproto::ClientMessageEvent) {
        ()
    }

    // a window has been destroyed, remove the corresponding client
    fn handle_destroy_notify(&mut self, ev: &xproto::DestroyNotifyEvent) {
        self.clients.remove(ev.window());
        self.reset_focus();
        self.arrange_windows();
        let pos = self
            .unmanaged_windows
            .iter()
            .position(|win| *win == ev.window());
        if let Some(index) = pos {
            println!("unregistered unmanaged window.");
            self.unmanaged_windows.swap_remove(index);
        }
    }

    // TODO: implement
    fn handle_configure_request(&self, _: &xproto::ConfigureRequestEvent) {
        ()
    }

    // a client has sent a map request - add it to the client set if it
    // hasn't been added yet and take necessary action to display it.
    fn handle_map_request(&mut self, ev: &xproto::MapRequestEvent) {
        let window = ev.window();
        // no client corresponding to the window, add it
        if self.clients.get_client_by_window(window).is_none() {
            // lookup properties of window
            let props = match self.get_properties(window) {
                Some(props) => props,
                None => {
                    println!("could not lookup properties.");
                    return;
                }
            };
            if props.window_type !=
                self.lookup_atom("_NET_WM_WINDOW_TYPE_DOCK") {
                // compute tags of the new client
                let tags = if let Some(res) = self.matching
                    .as_ref()
                    .and_then(|f| f(&props)) {
                    res
                } else if let Some(tagset) = self.tag_stack.current() {
                    tagset.tags.clone()
                } else {
                    vec![Tag::default()]
                };
                // map window
                let cookie = xproto::map_window(self.con, window);
                // set border width
                let cookie2 = xproto::configure_window(self.con, window,
                    &[(xproto::CONFIG_WINDOW_BORDER_WIDTH as u16,
                       self.config.border_width as u32)]);
                // create client object
                self.clients.add(Client::new(window, tags.clone(), props));
                if let Some(tagset) = self.tag_stack.current() {
                    if self.new_window_as_master() {
                        self.clients.swap_master(&tagset);
                    }
                }
                self.visible_windows.push(window);
                self.arrange_windows();
                self.reset_focus();
                if cookie.request_check().is_err() {
                    println!("could not map window.");
                }
                if cookie2.request_check().is_err() {
                    println!("could not set border width.");
                }
            } else {
                // it's a dock window - we don't care
                self.unmanaged_windows.push(ev.window());
                println!("registered unmanaged window.");
                if xproto::map_window(self.con, window)
                    .request_check().is_err() {
                    println!("could not map window.");
                }
            }
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
                Err(_) => {
                    return Err(WmError::CouldNotRegisterAtom(name.to_string()))
                }
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

    // get a window's properties (like window type and such)
    pub fn get_properties(&self, window: xproto::Window)
        -> Option<ClientProps> {
        let cookie1 =
            xproto::get_property(self.con,
                                 false,
                                 window,
                                 self.lookup_atom("_NET_WM_WINDOW_TYPE"),
                                 xproto::ATOM_ATOM,
                                 0,
                                 0xffffffff);
        let cookie2 = xproto::get_property(self.con,
                                           false,
                                           window,
                                           xproto::ATOM_WM_NAME,
                                           xproto::ATOM_STRING,
                                           0,
                                           0xffffffff);
        let cookie3 = xproto::get_property(self.con,
                                           false,
                                           window,
                                           xproto::ATOM_WM_CLASS,
                                           xproto::ATOM_STRING,
                                           0,
                                           0xffffffff);
        if let (Ok(r1), Ok(r2), Ok(r3)) = (cookie1.get_reply(),
                                           cookie2.get_reply(),
                                           cookie3.get_reply()) {
            unsafe {
                // we get exactly one atom
                let type_atoms: &[xproto::Atom] = transmute(r1.value());
                // the name is a single (variable-sized) string
                let name_slice: &[c_char] = transmute(r2.value());
                let name = CStr::from_ptr(name_slice.as_ptr())
                    .to_string_lossy();
                // the classes are a list of strings
                let class_slice: &[c_char] = transmute(r3.value());
                // iterate over them
                let mut class = Vec::new();
                for c in class_slice.split(|ch| *ch == 0) {
                    if c.len() > 0 {
                        if let Ok(cl) =
                               str::from_utf8(CStr::from_ptr(c.as_ptr())
                            .to_bytes()) {
                            class.push(cl.to_owned());
                        } else {
                            return None;
                        }
                    }
                }
                Some(ClientProps {
                    window_type: type_atoms[0].clone(),
                    name: name.into_owned(),
                    class: class,
                })
            }
        } else {
            None
        }
    }
}
