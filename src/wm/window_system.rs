use libc::c_char;

use std::collections::{HashMap, BTreeSet};
use std::ffi::CStr;
use std::process::exit;
use std::str;

use xcb::base;
use xcb::randr;
use xcb::xkb;
use xcb::xproto;

use wm::client::*;
use wm::config::{Tag, Mode, IGNORED_MODS_VEC};
use wm::err::*;
use wm::kbd::*;
use wm::layout::*;

/// Atoms we register with the X server for partial EWMH compliance.
static ATOM_VEC: [&'static str; 10] =
    ["WM_PROTOCOLS", "WM_DELETE_WINDOW", "_NET_WM_STATE",
     "WM_TAKE_FOCUS", "_NET_WM_NAME", "_NET_WM_CLASS",
     "_NET_WM_WINDOW_TYPE", "_NET_WM_WINDOW_TYPE_NORMAL",
     "_NET_WM_WINDOW_TYPE_DOCK", "_NET_WM_STATE_ABOVE"];

/// Association vector type for atoms and their names.
type AtomList<'a> = Vec<(xproto::Atom, &'a str)>;

/// Closure type of a callback function determining client placement on
/// creation.
///
/// Used to implement default tagsets for specific clients, as well as to
/// decide whether they appear as master windows or as slaves.
/// A value of `true` returned by the function as the second element of the
/// tuple signifies an insertion as a slave window, a value of `false`
/// indicates the window being inserted as a master window.
pub type Matching = Box<Fn(&ClientProps, &ScreenSet) -> Option<(BTreeSet<Tag>, bool)>>;

/// Closure type of a callback function modifying screen areas to configure
/// multimonitor setups and screen areas in general.
pub type ScreenMatching = Box<Fn(&mut Screen, randr::Crtc, usize)>;

/// Closure type of a callback function being called when a client sets it's urgent hint.
pub type UrgencyCallback = Box<Fn(&Client)>;

/// Enumeration type of commands executed by the window manager.
///
/// Being returned from a callback closure which modified internal structures,
/// gets interpreted to take necessary actions.
#[derive(Debug)]
pub enum WmCommand {
    /// redraw everything
    Redraw,
    /// reset focus
    Focus,
    /// kill the client associated with the window
    Kill(xproto::Window),
    /// switch keyboard mode
    ModeSwitch(Mode),
    /// change the current tagset's layout
    LayoutMsg(Vec<LayoutMessage>),
    /// replace the current tagset's layout
    LayoutSwitch(Box<Layout>),
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
    /// atoms registered at runtime
    atoms: AtomList<'a>,
    /// root window
    root: xproto::Window,
    /// the first event index of our RandR extension
    randr_base: u8,
    /// border width
    border_width: u8,
    /// a coordinate which is not visible in the current configuration
    safe_x: u32,
    /// colors used for window borders, first denotes focused windows
    border_colors: (u32, u32),
    /// all screen areas we tile windows on, and their tag stacks
    screens: ScreenSet,
    /// set of currently present clients
    clients: ClientSet,
    /// all windows currently visible
    visible_windows: Vec<xproto::Window>,
    /// windows we know about, but do not manage
    unmanaged_windows: Vec<xproto::Window>,
    /// currently focused window
    focused_window: Option<xproto::Window>,
    /// current keyboard mode
    mode: Mode,
    /// keybinding callbacks
    bindings: Keybindings,
    /// matching function for client placement
    matching: Option<Matching>,
    /// matching function for screen editing
    screen_matching: Option<ScreenMatching>,
    /// callback function for urgency handling
    urgency_callback: Option<UrgencyCallback>,
}

impl<'a> Wm<'a> {
    /// Wrap a connection to initialize a window manager.
    pub fn new(con: &'a base::Connection, screen_num: i32, config: WmConfig)
            -> Result<Wm<'a>, WmError> {
        if let Some(screen) = con.get_setup().roots().nth(screen_num as usize) {
            let root = screen.root();

            Ok(Wm {
                con: con,
                atoms: try!(get_atoms(con, &ATOM_VEC)),
                root: root,
                randr_base: 0,
                border_width: config.border_width,
                safe_x: screen.width_in_pixels() as u32,
                border_colors: try!(init_colors(con,
                                                screen.default_colormap(),
                                                config.f_color, config.u_color)),
                screens: try!(init_screens(con, root)),
                clients: ClientSet::default(),
                visible_windows: Vec::new(),
                unmanaged_windows: Vec::new(),
                focused_window: None,
                mode: Mode::default(),
                bindings: HashMap::new(),
                matching: None,
                screen_matching: None,
                urgency_callback: None,
            })
        } else {
            Err(WmError::CouldNotAcquireScreen)
        }
    }

    /// Initialize the RandR extension for multimonitor support.
    pub fn init_randr(&mut self) -> Result<(), WmError> {
        let values = randr::NOTIFY_MASK_CRTC_CHANGE
            | randr::NOTIFY_MASK_SCREEN_CHANGE;
        let cookie = randr::select_input(self.con, self.root, values as u16);
        let cookie2 = randr::query_version(self.con, 1, 2);
        let randr_query = self.con.get_extension_data(&mut randr::id());

        match (cookie.request_check(), cookie2.get_reply(), randr_query) {
            (Ok(()), Ok(ref r), Some(ref res)) =>
                if r.major_version() == 1 && r.minor_version() >= 2 {
                    self.randr_base = res.first_event();
                    info!("got RANDR base: {}", self.randr_base);
                    Ok(())
                } else {
                    Err(WmError::RandRVersionMismatch)
                },
            (Err(_), _, _) => Err(WmError::RandRSetupFailed),
            (_, Err(_), _) | (_, _, None) => Err(WmError::RandRVersionMismatch),
        }
    }

    /// Add all present clients to the datastructures on startup.
    pub fn init_clients(&mut self) {
        if let Ok(root) = xproto::query_tree(self.con, self.root).get_reply() {
            for window in root.children() {
                if let Ok((client, slave)) = self.construct_client(*window) {
                    self.add_client(client, slave);
                    self.visible_windows.push(*window);
                }
            }

            self.arrange_windows();
            self.reset_focus(true);
        }
    }

    /// Register window manager.
    ///
    /// Issues substructure redirects for the root window and registers for
    /// all events we are interested in.
    pub fn register(&mut self) -> Result<(), WmError> {
        let values = xproto::EVENT_MASK_SUBSTRUCTURE_REDIRECT
            | xproto::EVENT_MASK_SUBSTRUCTURE_NOTIFY;
        let cookie = xproto::change_window_attributes(
            self.con, self.root, &[(xproto::CW_EVENT_MASK, values)]);

        if cookie.request_check().is_ok() {
            Ok(())
        } else {
            Err(WmError::OtherWmRunning)
        }
    }

    /// Set up keybindings and necessary keygrabs.
    pub fn setup_bindings(&mut self, mut keys: Vec<(KeyPress, KeyCallback)>) {
        // compile keyboard bindings
        self.bindings.reserve(keys.len());
        for (key, callback) in keys.drain(..) {
            if self.bindings.insert(key, callback).is_some() {
                error!("overwriting bindings for a key");
            }
        }

        // minimize size of the bindings hashmap
        self.bindings.shrink_to_fit();

        // grab keys for the current mode
        self.grab_keys();
    }

    /// Grab the keys for the current mode.
    fn grab_keys(&self) {
        // don't grab anything for now
        if xproto::ungrab_key(self.con, xproto::GRAB_ANY as u8,
                              self.root, xproto::MOD_MASK_ANY as u16)
                .request_check().is_err() {
            error!("could not ungrab keys");
        }

        let cookies: Vec<_> =
            self.bindings
                .keys()
                .filter(|key| key.mode == self.mode)
                .flat_map(|key|
                    IGNORED_MODS_VEC
                        .iter()
                        .map(|modifier|
                            xproto::grab_key(
                                self.con, true, self.root,
                                *modifier | key.mods as u16, key.code,
                                xproto::GRAB_MODE_ASYNC as u8,
                                xproto::GRAB_MODE_ASYNC as u8)
                        )
                        .collect::<Vec<_>>()
                )
                .collect();

        // check for errors
        for cookie in cookies {
            if cookie.request_check().is_err() {
                error!("could not grab key");
            }
        }
    }

    /// Set up client matching.
    pub fn setup_matching(&mut self, matching: Matching) {
        self.matching = Some(matching);
    }

    /// Set up screen matching.
    pub fn setup_screen_matching(&mut self, matching: ScreenMatching) {
        self.screens.run_matching(&matching);
        self.screen_matching = Some(matching);
    }

    /// Set up urgency callback.
    pub fn setup_urgency_callback(&mut self, callback: UrgencyCallback) {
        self.urgency_callback = Some(callback);
    }

    /// Check whether we currently create new clients as masters or slaves.
    ///
    /// This depends on the layout of the currently viewed tagset.
    /// For instance, the `Monocle` layout only shows the master window,
    /// rendering client creation as a slave useless and unergonomic.
    fn new_window_as_master(&self) -> bool {
        self.screens
            .tag_stack()
            .current()
            .map_or(false, |tagset| tagset.layout.new_window_as_master())
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
        debug!("hidden windows: {:?}", self.visible_windows);
        // ... and reset the vector of visible windows
        self.visible_windows.clear();

        for &mut (_, ref mut screen) in self.screens.screens_mut() {
            if let Some(tagset) = screen.tag_stack.current() {
                // calculate next tag set ...
                let hidden = screen.tag_stack.get_hidden();
                let tags = tagset.tags.difference(hidden).cloned().collect();
                debug!("next batch of tags: {:?} \\ {:?} = {:?}",
                       tagset.tags, hidden, tags);

                // ... get the corresponding client set and geometries ...
                let clients = self.clients.get_order_or_insert(&tags);
                // TODO
                //let geometries = tagset.layout.arrange(clients.1.len(), &screen.area);
                //debug!("calculated geometries: {:?}", geometries);

                // ... and display windows accordingly
                //arrange(self.con, &mut self.visible_windows, clients, geometries);
            }
        }
    }

    /// Hide some windows by moving them offscreen.
    fn hide_windows(&self, windows: &[xproto::Window]) {
        let cookies: Vec<_> = windows
            .iter()
            .map(|window| xproto::configure_window(
                 self.con, *window,
                 &[(xproto::CONFIG_WINDOW_X as u16, self.safe_x),
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
        if !self.send_event(window, "WM_DELETE_WINDOW") {
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
    fn reset_focus(&mut self, draw_borders: bool) {
        let new =
            self.screens
                .tag_stack()
                .current()
                .and_then(|t| self.clients.get_focused_window(&t.tags))
                .unwrap_or(self.root);

        if self.new_window_as_master() && draw_borders {
           self.clients.swap_master(self.screens.tag_stack().current().unwrap());
           self.arrange_windows();
        }

        if draw_borders {
            if let Some(old_win) = self.focused_window {
                self.set_border_color(old_win, self.border_colors.1);
            }
        }

        if !self.send_event(new, "WM_TAKE_FOCUS") {
            info!("client didn't acept WM_TAKE_FOCUS message");
        }

        let cookie =
            xproto::set_input_focus(self.con,
                                    xproto::INPUT_FOCUS_POINTER_ROOT as u8,
                                    new,
                                    xproto::TIME_CURRENT_TIME);

        if draw_borders {
            self.set_border_color(new, self.border_colors.0);
        }

        if cookie.request_check().is_ok() {
            self.focused_window = Some(new);
        } else {
            error!("could not focus window");
        }
    }

    /// Color the borders of a window.
    fn set_border_color(&self, window: xproto::Window, color: u32) {
        let cookie =
            xproto::change_window_attributes(self.con, window,
                                             &[(xproto::CW_BORDER_PIXEL, color)]);

        if cookie.request_check().is_err() {
            error!("could not set window border color");
        }
    }

    /// Wait for events, handle them. Repeat.
    pub fn run(&mut self) -> Result<(), WmError> {
        loop {
            self.con.flush();
            if self.con.has_error().is_err() {
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
                self.handle_state_notify(base::cast_event(&event));
            },
            xproto::DESTROY_NOTIFY => {
                info!("received event: DESTROY_NOTIFY");
                self.handle_destroy_notify(base::cast_event(&event));
            },
            xproto::PROPERTY_NOTIFY => {
                self.handle_property_notify(base::cast_event(&event));
            },
            xproto::CONFIGURE_REQUEST => {
                info!("received event: CONFIGURE_REQUEST");
                self.handle_configure_request(base::cast_event(&event));
            },
            xproto::MAP_REQUEST => {
                info!("received event: MAP_REQUEST");
                self.handle_map_request(base::cast_event(&event));
            },
            res if res >= self.randr_base => match res - self.randr_base as u8 {
                randr::SCREEN_CHANGE_NOTIFY => {
                    info!("received event: SCREEN_CHANGE_NOTIFY");
                    self.handle_screen_change_notify(base::cast_event(&event));
                },
                randr::NOTIFY => {
                    info!("received event: CRTC_NOTIFY");
                    self.handle_crtc_notify(base::cast_event(&event));
                },
                _ => info!("ignoring event: {}", res),
            },
            res => info!("ignoring event: {}", res),
        }
    }

    /// The screen has been changed, react accordingly.
    ///
    /// If a rotation took place, make the geometries of our screens rotate.
    /// This might need some update in case we need to change some offsets as well.
    /// However, this code isn't likely to be used often.
    fn handle_screen_change_notify(&mut self, ev: &randr::ScreenChangeNotifyEvent) {
        if ev.root() != self.root {
            return;
        }

        if ev.rotation() as u32 &
                (randr::ROTATION_ROTATE_90 | randr::ROTATION_ROTATE_270) != 0 {
            info!("rotating all screen areas");
            self.screens.rotate();
        }

        self.safe_x = ev.width() as u32 + 2;
    }

    /// A crtc has been changed, react accordingly.
    fn handle_crtc_notify(&mut self, ev: &randr::NotifyEvent) {
        if ev.sub_code() as u32 == randr::NOTIFY_CRTC_CHANGE {
            let crtc_change: randr::CrtcChange = ev.u().cc();

            if crtc_change.mode() == 0 {
                info!("a crtc/screen removed from the screen set");
                if self.screens.remove(crtc_change.crtc()) {
                    self.arrange_windows();
                    self.reset_focus(true);
                }
            } else {
                self.screens.update(&crtc_change);
                info!("a crtc/screen from the screen set changed");
            }

            if let Some(ref matching) = self.screen_matching {
                info!("running screen matching");
                self.screens.run_matching(matching);
            }
        }
    }

    /// A key has been pressed, react accordingly.
    ///
    /// Look for a matching key binding upon event receival and call a
    /// callback closure if necessary. Determine what to do next based on
    /// the return value received.
    fn handle_state_notify(&mut self, ev: &xkb::StateNotifyEvent) {
        let key = from_key(ev, self.mode);
        let command = if let Some(func) = self.bindings.get(&key) {
            info!("executing binding for {:?}", key);
            let c = func(&mut self.clients, &mut self.screens);
            info!("resulting command: {:?}", c);
            c
        } else {
            WmCommand::NoCommand
        };

        match command {
            WmCommand::Redraw => {
                self.arrange_windows();
                self.reset_focus(true);
            },
            WmCommand::Focus => self.reset_focus(true),
            WmCommand::Kill(win) => self.destroy_window(win),
            WmCommand::ModeSwitch(mode) => {
                self.mode = mode;
                self.grab_keys();
            },
            WmCommand::LayoutMsg(msg) =>
                if self.screens
                    .tag_stack_mut()
                    .current_mut()
                    .map_or(false, |t| t.layout.edit_layout_retry(msg)) {
                    self.arrange_windows();
                },
            WmCommand::LayoutSwitch(layout) => {
                let matching = |t: &mut TagSet| { t.layout = layout; true };
                if self.screens
                    .tag_stack_mut()
                    .current_mut()
                    .map_or(false, matching) {
                    self.arrange_windows();
                }
            },
            WmCommand::Quit => exit(0),
            WmCommand::NoCommand => (),
        };
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
                self.visible_windows.swap_remove(index);
                self.arrange_windows();
            }
            self.reset_focus(true);
        } else {
            if let Some(index) = self
                    .unmanaged_windows
                    .iter()
                    .position(|win| *win == window) {
                self.unmanaged_windows.swap_remove(index);
                info!("unregistered unmanaged window");
            }
            self.reset_focus(false);
        }
    }

    /// A window updated some property, react accordingly.
    ///
    /// If said property was WM_HINTS, react to an urgency hint that is possibly set.
    fn handle_property_notify(&self, ev: &xproto::PropertyNotifyEvent) {
        use std::ops::Deref;
        if ev.atom() == xproto::ATOM_WM_HINTS {
            let window = ev.window();
            if let Some(client) = self
                    .clients
                    .get_client_by_window(window)
                    .and_then(|r| r.deref().try_borrow().ok()) {
                let hints = self.get_property_set(
                        window, vec![(xproto::ATOM_WM_HINTS, xproto::ATOM_WM_HINTS)]);
                if let Some(&ClientProp::PropAtom(ref res)) = hints.first() {
                    let not_focused =
                        self.screens
                            .current()
                            .tag_stack
                            .current()
                            .and_then(|tags| self.clients.get_focused_window(&tags.tags))
                            .map_or(false, |win| win != window);
                    match res.first() {
                        Some(res) if res & 0x100 != 0 && not_focused => {
                            info!("a client set it's urgency flag");
                            if let Some(ref callback) = self.urgency_callback {
                                callback(client.deref());
                            }
                        },
                        _ => (),
                    }
                }
            }
        }
    }

    /// A window wants to get a new geometry, react accordingly.
    ///
    /// If the window is managed (i.e. has a client), ignore the request.
    /// Otherwise, set it's geometry as desired.
    fn handle_configure_request(&self, ev: &xproto::ConfigureRequestEvent) {
        let window = ev.window();
        if self.clients.get_client_by_window(window).is_none() &&
                self.get_properties(window).window_type !=
                self.lookup_atom("_NET_WM_WINDOW_TYPE_DOCK") {
            let value_mask = ev.value_mask();
            let screen = self.screens.screen();
            let width = ev.width() as u32;
            let height = ev.height() as u32;
            let cookie =
                if value_mask as u32 & xproto::CONFIG_WINDOW_WIDTH != 0 &&
                        value_mask as u32 & xproto::CONFIG_WINDOW_HEIGHT != 0 &&
                        screen.width > width && screen.height > height {

                    let x = (screen.width - width) / 2;
                    let y = (screen.height - height) / 2;

                    let cookie = xproto::configure_window(
                        self.con, window,
                        &[(xproto::CONFIG_WINDOW_X as u16, x as u32),
                          (xproto::CONFIG_WINDOW_Y as u16, y as u32),
                          (xproto::CONFIG_WINDOW_WIDTH as u16, width),
                          (xproto::CONFIG_WINDOW_HEIGHT as u16, height)
                        ]);

                    info!("changing window geometry upon request: \
                          x={} y={} width={} height={}",
                          x, y, width, height);

                    cookie
                } else {
                    let mut x: u32 = 0;
                    let mut y: u32 = 0;

                    if let Ok(geom) =
                            xproto::get_geometry(self.con, window).get_reply() {
                        let width = geom.width() as u32;
                        let height = geom.height() as u32;
                        x = if screen.width > width {
                            (screen.width - width) / 2
                        } else {
                            0
                        };
                        y = if screen.height > height {
                            (screen.height - height) / 2
                        } else {
                            0
                        };
                    } else {
                        error!("could not get window geometry, expect ugly results");
                    }

                    let cookie = xproto::configure_window(
                        self.con, window,
                        &[(xproto::CONFIG_WINDOW_X as u16, x),
                          (xproto::CONFIG_WINDOW_Y as u16, y),
                        ]);

                    info!("changing window geometry upon request: x={} y={}", x, y);

                    cookie
                };

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
            match self.construct_client(window) {
                Ok((client, slave)) => {
                    // map window
                    let cookie = xproto::map_window(self.con, window);
                    // set border width and coordinates
                    let safe_x = self.screens.screen().width + 2;
                    let cookie2 = xproto::configure_window(self.con, window,
                        &[(xproto::CONFIG_WINDOW_BORDER_WIDTH as u16,
                           self.border_width as u32),
                          (xproto::CONFIG_WINDOW_X as u16, safe_x),
                          (xproto::CONFIG_WINDOW_Y as u16, 0)
                        ]);
                    let values = xproto::EVENT_MASK_PROPERTY_CHANGE;
                    let cookie3 = xproto::change_window_attributes(
                        self.con, window, &[(xproto::CW_EVENT_MASK, values)]);

                    // decide whether the client will be immediately visible
                    let visible =
                        self.screens
                            .tag_stack()
                            .current()
                            .map_or(false, |t| client.match_tags(&t.tags));

                    // add client to the necessary datastructures
                    self.add_client(client, slave);

                    // redraw currently visible clients if necessary
                    if visible {
                        debug!("new client is visible, arranging windows");
                        self.visible_windows.push(window);
                        self.arrange_windows();
                        self.reset_focus(true);
                    }

                    if cookie.request_check().is_err() {
                        error!("could not map window");
                    }
                    if cookie2.request_check().is_err() {
                        error!("could not set border width");
                    }
                    if cookie3.request_check().is_err() {
                        error!("could not register for client-specific events");
                    }
                }, // it's a window we don't care about
                Err(_) => self.register_unmanaged_window(window),
            }
        }
    }

    /// Initialize the state of a window we won't manage.
    fn register_unmanaged_window(&mut self, window: xproto::Window) {
        let cookie1 = xproto::map_window(self.con, window);
        let cookie2 = xproto::set_input_focus(
            self.con,
            xproto::INPUT_FOCUS_POINTER_ROOT as u8,
            window,
            xproto::TIME_CURRENT_TIME);

        self.unmanaged_windows.push(window);
        info!("registered unmanaged window");

        if cookie1.request_check().is_err() {
            error!("could not map window");
        }
        if cookie2.request_check().is_err() {
            error!("could not focus window");
        }
    }

    /// Construct a client for a window if we want to manage it.
    ///
    /// If the window has type `_NET_WM_WINDOW_TYPE_NORMAL`, and it hasn't set
    /// it's state to `_NET_WM_STATE_ABOVE`, generate a client structure for it
    /// and return it, otherwise don't.
    fn construct_client(&self, window: xproto::Window)
            -> Result<(Client, bool), ClientProps> {
        let props = self.get_properties(window);
        info!("props of new window: {:?}", props);

        let atom = self.lookup_atom("_NET_WM_STATE_ABOVE");
        if !props.state.iter().any(|s| *s == atom) &&
                props.window_type == self.lookup_atom("_NET_WM_WINDOW_TYPE_NORMAL") &&
                (!props.name.is_empty() || !props.class.is_empty()) {
            // compute tags of the new client
            let (tags, as_slave) = if let Some(res) = self.matching
                    .as_ref()
                    .and_then(|f| f(&props, &self.screens)) {
                res
            } else if let Some(tagset) = self.screens.tag_stack().current() {
                (tagset.tags.clone(), false)
            } else {
                (set![Tag::default()], false)
            };

            info!("client added on tags: {:?}", tags);

            Ok((Client::new(window, tags, props), as_slave))
        } else {
            Err(props)
        }
    }

    /// Add a client constructed from the parameters to the client store.
    ///
    /// Swaps new client with the master on the current layout if the
    /// currenlty used layout dictates it.
    fn add_client(&mut self, client: Client, as_slave: bool) {
        self.clients.add(client, as_slave);

        if let Some(tagset) = self.screens.tag_stack().current() {
            if self.new_window_as_master() {
                self.clients.swap_master(tagset);
            }
        }
    }

    /// Get an atom by name.
    fn lookup_atom(&self, name: &str) -> xproto::Atom {
        let index =
            self.atoms
                .iter()
                .position(|&(_, n)| n == name)
                .expect("unregistered atom used!");

        self.atoms[index].0
    }

    /// get a set of properties for a window, in parallel
    fn get_property_set(&self, window: xproto::Window,
                        atom_response_pairs: Vec<(xproto::Atom, xproto::Atom)>)
            -> Vec<ClientProp> {
        let cookies: Vec<_> = atom_response_pairs
            .iter()
            .map(|&(atom, response_type)|
                xproto::get_property(
                    self.con, false, window, atom, response_type, 0, 0xffffffff
                )
            )
            .collect();

        cookies
            .iter()
            .map(|cookie| if let Ok(reply) = cookie.get_reply() {
                match reply.type_() {
                    xproto::ATOM_ATOM => {
                        let atoms: &[xproto::Atom] = reply.value();
                        if atoms.len() == 0 {
                            ClientProp::NoProp
                        } else {
                            ClientProp::PropAtom(atoms.to_owned())
                        }
                    },
                    xproto::ATOM_WM_HINTS => {
                        let words: &[u32] = reply.value();
                        if words.len() == 0 {
                            ClientProp::NoProp
                        } else {
                            ClientProp::PropAtom(words.to_owned())
                        }
                    },
                    xproto::ATOM_STRING => {
                        let raw: &[c_char] = reply.value();
                        let mut res = Vec::new();
                        debug!("raw property data: {:?}, length: {}",
                               raw, reply.value_len());
                        for c in raw.split(|ch| *ch == 0) {
                            if c.len() > 0 {
                                unsafe {
                                    if let Ok(cl) = str::from_utf8(CStr::from_ptr(
                                            c.as_ptr()).to_bytes()) {
                                        res.push(cl.to_owned());
                                    } else {
                                        error!("decoding utf-8 from property failed");
                                    }
                                }
                            }
                        }
                        ClientProp::PropString(res)
                    },
                    _ => ClientProp::NoProp,
                }
            } else {
                error!("could not look up property");
                ClientProp::NoProp
            })
            .collect()
    }

    /// Get a window's properties (like window type and such), if possible.
    fn get_properties(&self, window: xproto::Window) -> ClientProps {
        let mut properties = self.get_property_set(window, vec![
            (self.lookup_atom("_NET_WM_WINDOW_TYPE"), xproto::ATOM_ATOM),
            (self.lookup_atom("_NET_WM_STATE"), xproto::ATOM_ATOM),
            (xproto::ATOM_WM_NAME, xproto::ATOM_STRING),
            (self.lookup_atom("_NET_WM_NAME"), xproto::ATOM_STRING),
            (xproto::ATOM_WM_CLASS, xproto::ATOM_STRING),
            (self.lookup_atom("_NET_WM_CLASS"), xproto::ATOM_STRING)
        ]);
        let mut props = properties.drain(..);

        let window_type = if let Some(ClientProp::PropAtom(mut t)) = props.next() {
            t.drain(..).next().unwrap_or(self.lookup_atom("_NET_WM_WINDOW_TYPE_NORMAL"))
        } else { // assume reasonable default
            info!("_NET_WM_WINDOW_TYPE: not set, assuming _NET_WM_WINDOW_TYPE_NORMAL");
            self.lookup_atom("_NET_WM_WINDOW_TYPE_NORMAL")
        };

        let state = match props.next() {
            Some(ClientProp::PropAtom(s)) => s,
            Some(ClientProp::NoProp) => {
                info!("_NET_WM_STATE: not set");
                Vec::new()
            },
            _ => {
                error!("_NET_WM_STATE: unexpected response type");
                Vec::new()
            },
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

        let class2 = match props.next() {
            Some(ClientProp::PropString(c)) => c,
            Some(ClientProp::NoProp) => {
                info!("_NET_WM_CLASS: not set");
                Vec::new()
            },
            _ => {
                error!("_NET_WM_CLASS: unexpected response type");
                Vec::new()
            },
        };

        class.extend(class2);

        ClientProps {
            window_type: window_type,
            state: state,
            name: if name2.is_empty() { name } else { name2 },
            class: class,
        }
    }

    /// Send an atomic event to a client specified by a window.
    ///
    /// Returns the error status of the event sent.
    fn send_event(&self, window: xproto::Window, atom: &'static str) -> bool {
        let data = [self.lookup_atom(atom), 0, 0, 0, 0].as_ptr()
            as *const xproto::ClientMessageData;
        let event = unsafe {
            xproto::ClientMessageEvent::new(
                32, window, self.lookup_atom("WM_PROTOCOLS"), *data)
        };

        xproto::send_event(self.con, false, window,
                           xproto::EVENT_MASK_NO_EVENT, &event)
            .request_check()
            .is_ok()
    }
}

/// Allocate colors needed for border drawing.
fn init_colors(con: &base::Connection, colormap: xproto::Colormap,
               f_color: (u16, u16, u16), u_color: (u16, u16, u16))
        -> Result<(u32, u32), WmError> {
    // request color pixels
    let f_cookie = xproto::alloc_color(con, colormap, f_color.0, f_color.1, f_color.2);
    let u_cookie = xproto::alloc_color(con, colormap, u_color.0, u_color.1, u_color.2);

    // get the replies
    match (f_cookie.get_reply(), u_cookie.get_reply()) {
        (Ok(f_reply), Ok(u_reply)) => Ok((f_reply.pixel(), u_reply.pixel())),
        _ => Err(WmError::CouldNotAllocateColors),
    }
}

// Get info on all outputs and register them in a `ScreenSet`.
fn init_screens(con: &base::Connection, root: xproto::Window)
        -> Result<ScreenSet, WmError> {
    if let Ok(reply) = randr::get_screen_resources(con, root).get_reply() {
        let cfg = reply.config_timestamp();
        let cookies: Vec<_> = reply.crtcs()
            .iter()
            .map(|crtc| (crtc, randr::get_crtc_info(con, *crtc, cfg)))
            .collect();
        let screens = cookies
            .iter()
            .filter_map(|&(crtc, ref cookie)| if let Ok(r) = cookie.get_reply() {
                let width = r.width() as u32;
                let height = r.height() as u32;
                if width > 0 && height > 0 {
                    let tiling_area =
                        TilingArea {
                            offset_x: r.x() as u32,
                            offset_y: r.y() as u32,
                            width: width,
                            height: height,
                        };
                    Some((*crtc, Screen::new(tiling_area, TagStack::default())))
                } else {
                    None
                }
            } else {
                None
            })
            .collect();

        if let Some(res) = ScreenSet::new(screens) {
            Ok(res)
        } else {
            Err(WmError::BadCrtc)
        }
    } else {
        Err(WmError::CouldNotGetScreenResources)
    }
}

/// Register and get back atoms, return an error on failure.
fn get_atoms<'a>(con: &base::Connection, names: &[&'a str])
        -> Result<Vec<(xproto::Atom, &'a str)>, WmError> {
    let len = names.len();
    let cookies: Vec<_> = names
        .iter()
        .map(|name| (xproto::intern_atom(con, false, name), *name))
        .collect();

    let mut res = Vec::with_capacity(len);
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

/// Rearrange windows according to the geometries provided.
///
/// This is the parallel version running each request-reply in an interleaved fashion.
#[cfg(feature = "parallel-resizing")]
fn arrange(con: &base::Connection,
           visible: &mut Vec<xproto::Window>,
           clients: &OrderedSubset,
           geometries: Vec<Option<Geometry>>) {
    // TODO
    /*let cookies: Vec<_> = clients.1
        .iter()
        .zip(geometries.iter())
        .filter_map(|(client, geometry)|
            if let (Some(ref cl), &Some(ref geom)) = (client.upgrade(), geometry) {
                Some((cl.borrow().window, geom))
            } else {
                None
            }
        )
        .map(|(window, geometry)|
            (xproto::configure_window(
                con, window,
                &[(xproto::CONFIG_WINDOW_X as u16, geometry.x as u32),
                  (xproto::CONFIG_WINDOW_Y as u16, geometry.y as u32),
                  (xproto::CONFIG_WINDOW_WIDTH as u16,
                   geometry.width as u32),
                  (xproto::CONFIG_WINDOW_HEIGHT as u16,
                   geometry.height as u32)
                ]), window)
        )
        .collect();

    for (cookie, window) in cookies {
        // we do this here to avoid ugly issues with lifetimes
        visible.push(window);
        if cookie.request_check().is_err() {
            error!("could not set window geometry");
        }
    }*/
}

/// Rearrange windows according to the geometries provided.
///
/// This is the sequential version running each request-reply pair after the other.
#[cfg(not(feature = "parallel-resizing"))]
fn arrange(con: &base::Connection,
           visible: &mut Vec<xproto::Window>,
           clients: &OrderEntry,
           geometries: Vec<Option<Geometry>>) {
    for (client, geometry) in clients.1.iter().zip(geometries.iter()) {
        if let (Some(ref cl), &Some(ref geom)) = (client.upgrade(), geometry) {
            let window = cl.borrow().window;
            visible.push(window);
            let cookie = xproto::configure_window(
                con, window,
                &[(xproto::CONFIG_WINDOW_X as u16, geom.x as u32),
                  (xproto::CONFIG_WINDOW_Y as u16, geom.y as u32),
                  (xproto::CONFIG_WINDOW_WIDTH as u16,
                   geom.width as u32),
                  (xproto::CONFIG_WINDOW_HEIGHT as u16,
                   geom.height as u32)
                ]);

            if cookie.request_check().is_err() {
                error!("could not set window geometry");
            }
        }
    }
}
