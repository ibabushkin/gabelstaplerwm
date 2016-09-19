use libc::c_char;

use std::collections::HashMap;
use std::ffi::CStr;
use std::process::exit;
use std::str;

use xcb::base;
use xcb::xkb;
use xcb::xproto;
use xcb::ffi::xcb_client_message_data_t;

use wm::client::*;
use wm::config::{Tag,Mode};
use wm::err::*;
use wm::kbd::*;
use wm::layout::*;

/// Atoms we register with the X server for partial EWMH compliance.
static ATOM_VEC: [&'static str; 10] =
    ["WM_PROTOCOLS", "WM_DELETE_WINDOW", "_NET_WM_STATE",
     "WM_TAKE_FOCUS", "_NET_WM_TAKE_FOCUS", "_NET_WM_NAME", "_NET_WM_CLASS",
     "_NET_WM_WINDOW_TYPE", "_NET_WM_WINDOW_TYPE_NORMAL",
     "_NET_WM_STATE_ABOVE"];

/// Association vector type for atoms and their names.
type AtomList<'a> = Vec<(xproto::Atom, &'a str)>;

/// Closure type of a callback function determining client placement on
/// creation.
///
/// Used to implement default tagsets for specific clients.
pub type Matching = Box<Fn(&ClientProps) -> Option<Vec<Tag>>>;

/// Enumeration type of commands executed by the window manager.
///
/// Being returned from a callback closure which modified internal structures,
/// gets interpreted to take necessary actions.
#[derive(Debug, PartialEq, Eq)]
pub enum WmCommand {
    /// redraw everything
    Redraw,
    /// reset focus
    Focus,
    /// kill the client associated with the window
    Kill(xproto::Window),
    /// switch keyboard mode
    ModeSwitch(Mode),
    /// quit window manager
    Quit,
    /// don't do anything, no action is needed
    NoCommand,
}

/// Configuration information used by the window manager.
#[derive(Clone)]
pub struct WmConfig {
    /// color of focused window's border
    pub f_color: (u16, u16, u16),
    /// color of unfocused window's border
    pub u_color: (u16, u16, u16),
    /// window border width
    pub border_width: u8,
    /// screen parameters requested by user
    pub screen: ScreenSize,
}

/// A window manager master-structure.
///
/// This is the central instance coordinating the communication
/// with the X server, as well as containing structures to manage tags
/// and clients. It also contains callback mechanisms upon key press and
/// client creation.
pub struct Wm<'a> {
    /// connection to the X server
    con: &'a base::Connection,
    /// root window
    root: xproto::Window,
    /// user-defined configuration parameters
    config: WmConfig,
    /// screen parameters as obtained from the X server upon connection
    screen: ScreenSize,
    /// colors used for window borders, first denotes focused windows
    border_colors: (u32, u32),
    /// keybinding callbacks
    bindings: Keybindings,
    /// matching function for client placement
    matching: Option<Matching>,
    /// current keyboard mode
    mode: Mode,
    /// set of currently present clients
    clients: ClientSet,
    /// set of currently present tagsets and their display history
    tag_stack: TagStack,
    /// atoms registered at runtime
    atoms: AtomList<'a>,
    /// all windows currently visible
    visible_windows: Vec<xproto::Window>,
    /// currently focused window
    focused_window: Option<xproto::Window>,
    /// windows we know about, but do not manage
    unmanaged_windows: Vec<xproto::Window>,
}

impl<'a> Wm<'a> {
    /// Wrap a connection to initialize a window manager.
    pub fn new(con: &'a base::Connection, screen_num: i32, config: WmConfig)
        -> Result<Wm<'a>, WmError> {
        let setup = con.get_setup();
        if let Some(screen) = setup.roots().nth(screen_num as usize) {
            let width = screen.width_in_pixels();
            let height = screen.height_in_pixels();
            let colormap = screen.default_colormap();
            let new_screen =
                ScreenSize::new(&config.screen, width as u32, height as u32);
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
                        mode: Mode::default(),
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

    /// Allocate colors needed for border drawing.
    fn setup_colors(con: &'a base::Connection,
                    colormap: xproto::Colormap,
                    f_color: (u16, u16, u16),
                    u_color: (u16, u16, u16))
        -> (u32, u32) {
        // request color pixels
        let f_cookie = xproto::alloc_color(
            con, colormap, f_color.0, f_color.1, f_color.2);
        let u_cookie = xproto::alloc_color(
            con, colormap, u_color.0, u_color.1, u_color.2);

        // get the replies
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

    /// Register window manager.
    ///
    /// Issues substructure redirects for the root window and registers for
    /// all events we are interested in.
    pub fn register(&self) -> Result<(), WmError> {
        let values = xproto::EVENT_MASK_SUBSTRUCTURE_REDIRECT
            | xproto::EVENT_MASK_SUBSTRUCTURE_NOTIFY
            | xproto::EVENT_MASK_PROPERTY_CHANGE;
        match xproto::change_window_attributes(
            self.con, self.root, &[(xproto::CW_EVENT_MASK, values)])
            .request_check() {
            Ok(()) => Ok(()),
            Err(_) => Err(WmError::OtherWmRunning),
        }
    }

    /// Set up keybindings and necessary keygrabs.
    pub fn setup_bindings(&mut self, mut keys: Vec<(KeyPress, KeyCallback)>) {
        // don't grab anything for now
        xproto::ungrab_key(
            self.con, xproto::GRAB_ANY as u8,
            self.root, xproto::MOD_MASK_ANY as u16
        );

        // compile keyboard bindings
        self.bindings = HashMap::with_capacity(keys.len());
        let cookies: Vec<_> = keys
            .drain(..)
            .filter_map(|(key, callback)|
                if self.bindings.insert(key, callback).is_some() {
                    error!("overwriting bindings for a key!");
                    None
                } else {
                    // register for the corresponding event
                    Some(xproto::grab_key(
                        self.con, true, self.root,
                        key.mods as u16, key.code,
                        xproto::GRAB_MODE_ASYNC as u8,
                        xproto::GRAB_MODE_ASYNC as u8
                    ))
                }
            )
            .collect();

        // check for errors
        for cookie in cookies {
            if cookie.request_check().is_err() {
                error!("could not grab key!");
            }
        }
    }

    /// Set up client matching.
    pub fn setup_matching(&mut self, matching: Matching) {
        self.matching = Some(matching);
    }

    /// Set up the tagset stack.
    pub fn setup_tags(&mut self, stack: TagStack) {
        self.tag_stack = stack;
    }

    /// Add all present clients to the datastructures on startup.
    pub fn setup_clients(&mut self) {
        if let Ok(root) = xproto::query_tree(self.con, self.root).get_reply() {
            for window in root.children() {
                if let Some(client) = self.construct_client(*window) {
                    self.add_client(client);
                    self.visible_windows.push(*window);
                }
            }
            self.arrange_windows();
            self.reset_focus();
        }
    }

    /// Check whether we currently create new clients as masters or slaves.
    ///
    /// This depends on the layout of the currently viewed tagset.
    /// For instance, the `Monocle` layout only shows the master window,
    /// rendering client creation as a slave useless and unergonomic.
    fn new_window_as_master(&self) -> bool {
        match self.tag_stack.current() {
            Some(tagset) => tagset.layout.new_window_as_master(),
            _ => false,
        }
    }

    /// Using the current layout, arrange all visible windows.
    ///
    /// This first determines the set of visible windows, and displays them
    /// accordingly after hiding all windows. This semantic was chosen, because
    /// redraws are only triggered when the set of visible windows is expected
    /// to have changed, e.g. when a user-defined callback returned the
    /// corresponding `WmCommand`.
    fn arrange_windows(&mut self) {
        // first, hide all visible windows ...
        self.hide_windows(&self.visible_windows);
        // ... and reset the vector of visible windows
        self.visible_windows.clear();
        // setup current client list
        let (clients, layout) = match self.tag_stack.current() {
            Some(tagset) => (
                self.clients.get_order_or_insert(&tagset.tags),
                &tagset.layout
            ),
            None => return, // nothing to do here - no current tagset
        };
        // get geometries ...
        let geometries = layout.arrange(clients.1.len(), &self.screen);
        // we set geometries in serial, because otherwise window redraws are
        // rendered lazily, at least with xephyr. to avoid this condition,
        // we accept some additional waiting time, which doesn't matter much
        // - redraw times aren't subject to visible latency anyway. until this
        // is fixed, the code below has to stay serial in nature.
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
                    error!("could not set window geometry");
                }
            }
        }
    }

    /// Hide some windows by moving them offscreen.
    fn hide_windows(&self, windows: &[xproto::Window]) {
        let safe_x = (self.screen.width * 2) as u32;
        let cookies: Vec<_> = windows
            .iter()
            .map(|window| xproto::configure_window(
                 self.con, *window,
                 &[(xproto::CONFIG_WINDOW_X as u16, safe_x),
                   (xproto::CONFIG_WINDOW_Y as u16, 0)]
                )
            )
            .collect();
        for cookie in cookies {
            if cookie.request_check().is_err() {
                error!("could not move window offscreen");
            }
        }

    }

    /// Destroy a window.
    ///
    /// Send a client message and kill the client the hard and merciless way
    /// if that fails, for instance if the client ignores such messages.
    fn destroy_window(&self, window: xproto::Window) {
        if self.send_event(window, "WM_DELETE_WINDOW") {
            info!("client didn't accept WM_DELETE_WINDOW message");
            if xproto::kill_client(self.con, window).request_check().is_err() {
                error!("could not kill client");
            }
        }
    }

    /// Reset focus.
    ///
    /// The datastructures have been altered, we need to focus the appropriate
    /// window as obtained from there. If an old window is present, uncolor
    /// it's border.
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
            if self.send_event(new, "WM_TAKE_FOCUS") {
                info!("client didn't acept WM_TAKE_FOCUS message");
            }
            if self.send_event(new, "_NET_WM_TAKE_FOCUS") {
                info!("client didn't acept _NET_WM_TAKE_FOCUS message");
            }
            let cookie =
                xproto::set_input_focus(self.con,
                                        xproto::INPUT_FOCUS_POINTER_ROOT as u8,
                                        new,
                                        xproto::TIME_CURRENT_TIME);
            self.set_border_color(new, self.border_colors.0);
            if cookie.request_check().is_err() {
                error!("could not focus window");
            } else {
                self.focused_window = Some(new);
            }
        }
    }

    /// Color the borders of a window.
    fn set_border_color(&self, window: xproto::Window, color: u32) {
        let cookie = xproto::change_window_attributes(
            self.con, window, &[(xproto::CW_BORDER_PIXEL, color)]);
        if cookie.request_check().is_err() {
            error!("could not set window border color");
        }
    }

    /// Wait for events, handle them. Repeat.
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

    /// Handle an event received from the X server.
    fn handle(&mut self, event: base::GenericEvent) {
        match event.response_type() {
            xkb::STATE_NOTIFY => {
                info!("received event: STATE_NOTIFY");
                self.handle_state_notify(base::cast_event(&event))
            },
            xproto::PROPERTY_NOTIFY => {
                info!("received event: PROPERTY_NOTIFY");
                self.handle_property_notify(base::cast_event(&event))
            },
            xproto::CLIENT_MESSAGE => {
                info!("received event: CLIENT_MESSAGE");
                self.handle_client_message(base::cast_event(&event))
            },
            xproto::DESTROY_NOTIFY => {
                info!("received event: DESTROY_NOTIFY");
                self.handle_destroy_notify(base::cast_event(&event))
            },
            xproto::CONFIGURE_REQUEST => {
                info!("received event: CONFIGURE_REQUEST");
                self.handle_configure_request(base::cast_event(&event))
            },
            xproto::MAP_REQUEST => {
                info!("received event: MAP_REQUEST");
                self.handle_map_request(base::cast_event(&event))
            },
            num => info!("ignoring event: {}", num),
        }
    }

    /// A key has been pressed, react accordingly.
    ///
    /// Look for a matching key binding upon event receival and call a callback
    /// closure if necessary. Determine what to do next based on the
    /// return value received.
    fn handle_state_notify(&mut self, ev: &xkb::StateNotifyEvent) {
        let key = from_key(ev, self.mode);
        let command = if let Some(func) = self.bindings.get(&key) {
            info!("executing binding for {:?}", key);
            let c = func(&mut self.clients, &mut self.tag_stack);
            info!("resulting command: {:?}", c);
            c
        } else {
            WmCommand::NoCommand
        };
        match command {
            WmCommand::Redraw => {
                self.arrange_windows();
                self.reset_focus();
            },
            WmCommand::Focus => self.reset_focus(),
            WmCommand::Kill(win) => self.destroy_window(win),
            WmCommand::ModeSwitch(mode) => self.mode = mode,
            WmCommand::Quit => exit(0),
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

    /// A window has been destroyed, react accordingly.
    ///
    /// If the window is managed (i.e. has a client), destroy it. Otherwise,
    /// remove it from the vector of unmanaged windows.
    fn handle_destroy_notify(&mut self, ev: &xproto::DestroyNotifyEvent) {
        let window = ev.window();
        if self.clients.remove(window) {
            if let Some(index) = self
                .visible_windows
                .iter()
                .position(|win| *win == window) {
                self.reset_focus();
                self.visible_windows.swap_remove(index);
                self.arrange_windows();
            }
        }
        if let Some(index) = self
            .unmanaged_windows
            .iter()
            .position(|win| *win == window) {
            self.unmanaged_windows.swap_remove(index);
            info!("unregistered unmanaged window");
        }
    }

    /// A window wants to get a new geometry, react accordingly.
    ///
    /// If the window is managed (i.e. has a client), ignore the request.
    /// Otherwise, set it's geometry as desired.
    fn handle_configure_request(&self, ev: &xproto::ConfigureRequestEvent) {
        let window = ev.window();
        if self.clients.get_client_by_window(window).is_none() {
            info!("changing window geometry upon request");
            let cookie = xproto::configure_window(
                self.con, window,
                &[(xproto::CONFIG_WINDOW_X as u16, ev.x() as u32),
                  (xproto::CONFIG_WINDOW_Y as u16, ev.y() as u32),
                  (xproto::CONFIG_WINDOW_WIDTH as u16, ev.width() as u32),
                  (xproto::CONFIG_WINDOW_HEIGHT as u16, ev.height() as u32)
                ]);
            if cookie.request_check().is_err() {
                error!("could not set window geometry");
            }
        }
    }

    /// A client has sent a map request, react accordingly.
    ///
    /// Add the window to the necessary structures if it is not yet known and
    /// all prerequisitory conditions are met.
    fn handle_map_request(&mut self, ev: &xproto::MapRequestEvent) {
        let window = ev.window();
        // no client corresponding to the window, add it
        if self.clients.get_client_by_window(window).is_none() {
            if let Some(client) = self.construct_client(window) {
                // map window
                let cookie = xproto::map_window(self.con, window);
                // set border width
                let cookie2 = xproto::configure_window(self.con, window,
                    &[(xproto::CONFIG_WINDOW_BORDER_WIDTH as u16,
                       self.config.border_width as u32)]);
                self.add_client(client);
                self.visible_windows.push(window);
                self.arrange_windows();
                self.reset_focus();
                if cookie.request_check().is_err() {
                    error!("could not map window");
                }
                if cookie2.request_check().is_err() {
                    error!("could not set border width");
                }
            } else {
                // it's a window we don't care about
                let cookie = xproto::map_window(self.con, window);
                let cookie2 = xproto::set_input_focus(
                    self.con,
                    xproto::INPUT_FOCUS_POINTER_ROOT as u8,
                    window,
                    xproto::TIME_CURRENT_TIME);
                self.set_border_color(window, self.border_colors.0);
                self.add_unmanaged(window);
                if cookie.request_check().is_err() {
                    error!("could not map window");
                }
                if cookie2.request_check().is_err() {
                    error!("could not focus window");
                }
            }
        }
    }

    /// Construct a client for a window if we want to manage it.
    ///
    /// If the window has type `_NET_WM_WINDOW_TYPE_NORMAL`,
    /// generate a client structure for it and return it, otherwise don't.
    fn construct_client(&self, window: xproto::Window) -> Option<Client> {
        let props = self.get_properties(window);
        info!("props of new window: {:?}", props);
        if props.state != Some(self.lookup_atom("_NET_WM_STATE_ABOVE")) &&
            props.name != "" &&
            props.window_type == self.lookup_atom("_NET_WM_WINDOW_TYPE_NORMAL") {
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
            info!("client added on tags: {:?}", tags);
            Some(Client::new(window, tags, props))
        } else {
            None
        }
    }

    /// Add a client constructed from the parameters to the client store.
    ///
    /// Swaps new client with the master on the current layout if the
    /// currenlty used layout dictates it.
    fn add_client(&mut self, client: Client) {
        self.clients.add(client);
        if let Some(tagset) = self.tag_stack.current() {
            if self.new_window_as_master() {
                self.clients.swap_master(tagset);
            }
        }
    }

    /// Add a window to the list of unmanaged windows.
    fn add_unmanaged(&mut self, window: xproto::Window) {
        self.unmanaged_windows.push(window);
        info!("registered unmanaged window");
    }

    /// Register and get back atoms, return an error on failure.
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

    /// Get an atom by name.
    fn lookup_atom(&self, name: &str) -> xproto::Atom {
        self.atoms[
            self.atoms
                .iter()
                .position(|&(_, n)| n == name)
                .expect("unregistered atom used!")
        ].0
    }

    /// get a set of properties for a window, in parallel
    fn get_property_set(&self, window: xproto::Window,
                        atom_response_pairs: Vec<(xproto::Atom, xproto::Atom)>)
        -> Vec<ClientProp> {
        let mut cookies: Vec<_> = atom_response_pairs
            .iter()
            .map(|&(atom, response_type)|
                xproto::get_property(
                    self.con, false, window, atom, response_type, 0, 0xffffffff
                )
            )
            .collect();
        let res = cookies
            .drain(..)
            .zip(atom_response_pairs.iter())
            .map(|(cookie, &(_, response_type))| // TODO: use the returned response type
                match response_type {
                    xproto::ATOM_ATOM => if let Ok(r) = cookie.get_reply() {
                        let atoms: &[xproto::Atom] = r.value();
                        if atoms.len() == 0 {
                            ClientProp::NoProp
                        } else {
                            ClientProp::PropAtom(atoms[0])
                        }
                    } else {
                        ClientProp::NoProp
                    },
                    xproto::ATOM_STRING => if let Ok(r) = cookie.get_reply() {
                        let raw: &[c_char] = r.value();
                        let mut res = Vec::new();
                        debug!("raw property data: {:?}, length: {}, type: {}",
                               raw, r.value_len(), r.type_());
                        for c in raw.split(|ch| *ch == 0) {
                            if c.len() > 0 {
                                unsafe {
                                    if let Ok(cl) = str::from_utf8(
                                        CStr::from_ptr(c.as_ptr()).to_bytes()) {
                                        res.push(cl.to_owned());
                                    } else {
                                        error!("decoding utf-8 from property failed");
                                    }
                                }
                            }
                        }
                        ClientProp::PropString(res)
                    } else {
                        ClientProp::NoProp
                    },
                    _ => ClientProp::NoProp,
                }
            )
            .collect();
        res
    }

    /// Get a window's properties (like window type and such), if possible.
    pub fn get_properties(&self, window: xproto::Window) -> ClientProps {
        let mut properties = self.get_property_set(window, vec![
            (self.lookup_atom("_NET_WM_WINDOW_TYPE"), xproto::ATOM_ATOM),
            (self.lookup_atom("_NET_WM_STATE"), xproto::ATOM_ATOM),
            (xproto::ATOM_WM_NAME, xproto::ATOM_STRING),
            (self.lookup_atom("_NET_WM_NAME"), xproto::ATOM_STRING),
            (xproto::ATOM_WM_CLASS, xproto::ATOM_STRING),
            (self.lookup_atom("_NET_WM_CLASS"), xproto::ATOM_STRING)
        ]);
        let mut props = properties.drain(..);

        let window_type = if let Some(ClientProp::PropAtom(t)) = props.next() {
            t
        } else { // assume reasonable default
            info!("_NET_WM_WINDOW_TYPE: not set, assuming _NET_WM_WINDOW_TYPE_NORMAL");
            self.lookup_atom("_NET_WM_WINDOW_TYPE_NORMAL")
        };

        let state_iter = props.next();
        let state = if let Some(ClientProp::PropAtom(s)) = state_iter {
            Some(s)
        } else {
            if state_iter == Some(ClientProp::NoProp) {
                info!("_NET_WM_STATE: not set");
            } else {
                error!("_NET_WM_STATE: unexpected response type");
            }
            None
        };

        let name = if let Some(ClientProp::PropString(mut n)) = props.next() {
            if n.len() >= 1 {
                n.remove(0)
            } else {
                error!("WM_NAME: no value(s)");
                String::new()
            }
        } else {
            error!("WM_NAME: unexpected or no response type");
            String::new()
        };

        let name2 = if let Some(ClientProp::PropString(mut n)) = props.next() {
            if n.len() >= 1 {
                n.remove(0)
            } else {
                error!("_NET_WM_NAME: no value(s)");
                String::new()
            }
        } else {
            error!("_NET_WM_NAME: unexpected or no response type");
            String::new()
        };

        let mut class = if let Some(ClientProp::PropString(c)) = props.next() {
            c
        } else {
            error!("WM_CLASS: unexpected or no response type");
            Vec::new()
        };

        let class2_iter = props.next();
        let class2 = if let Some(ClientProp::PropString(c)) = class2_iter {
            c
        } else {
            if class2_iter == Some(ClientProp::NoProp) {
                info!("_NET_WM_CLASS: not set");
            } else {
                error!("_NET_WM_CLASS: unexpected response type");
            }
            Vec::new()
        };

        class.extend(class2);

        ClientProps {
            window_type: window_type,
            state: state,
            name: if name2 == "" { name } else { name2 },
            class: class,
        }
    }

    /// Send an atomic event to a client specified by a window.
    fn send_event(&self, window: xproto::Window, atom: &'static str) -> bool {
        let data = [self.lookup_atom(atom), 0, 0, 0, 0].as_ptr()
            as *const xcb_client_message_data_t;
        let event = unsafe {
            xproto::ClientMessageEvent::new(
                32, window, self.lookup_atom("WM_PROTOCOLS"), *data)
        };
        xproto::send_event(self.con, false, window,
                           xproto::EVENT_MASK_NO_EVENT, &event)
            .request_check()
            .is_err()
    }
}
