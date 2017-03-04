//! # gabelstaplerwm - a semidynamic tiling window manager
//! It's what the heading says it is. The documentation found here is a very
//! dense description of what the sources do. It is intended to be read along
//! with the sources, as the configuration process involves you editing them.
//! See the documentation for the `config` module for more information on
//! configuration.

#![feature(slice_patterns)]

extern crate env_logger;
extern crate vec_arena;
extern crate libc;
#[macro_use]
extern crate log;
extern crate xcb;

use std::env::remove_var;
use std::mem::{transmute, uninitialized};

pub mod wm;
use wm::config::*;
use wm::err::*;
use wm::window_system::Wm;

use xcb::base::*;

/// Reap children.
extern fn sigchld_action(_: libc::c_int) {
    unsafe {
        loop {
            let pid = libc::waitpid(-1, 0 as *mut libc::c_int, libc::WNOHANG);
            if pid <= 0 {
                return;
            }
        }
    }
}

/// Main function.
///
/// Sets up connection, and window manager object.
/// Glue code to include user config.
fn main() {
    // logger setup
    if env_logger::init().is_err() {
        handle_logger_error();
    }

    // we're a good parent - we wait for our children when they get a screaming
    // fit at the checkout lane
    unsafe {
        // initialize the sigaction struct
        let mut act = uninitialized::<libc::sigaction>();

        // convert our handler to a C-style function pointer
        let f_ptr: *const libc::c_void =
            transmute(sigchld_action as extern fn(libc::c_int));
        act.sa_sigaction = f_ptr as libc::sighandler_t;

        // some default values noone cares about
        libc::sigemptyset(&mut act.sa_mask);
        act.sa_flags = libc::SA_RESTART;

        // setup our SIGCHLD-handler
        if libc::sigaction(libc::SIGCHLD, &act, 0 as *mut libc::sigaction) == -1 {
            // crash and burn on failure
            WmError::CouldNotEstablishHandlers.handle();
        }
    }

    // clean environment for cargo and other processes honoring `RUST_LOG`
    remove_var("RUST_LOG");

    // include user config
    let config = generate_config();

    // create new connection to X server
    let (con, screen_num) = match Connection::connect(None) {
        Ok(c) => c,
        Err(e) => WmError::CouldNotConnect(e).handle(),
    };

    // initialize window manager
    let mut wm = match Wm::new(&con, screen_num, config) {
        Ok(w) => w,
        Err(e) => e.handle(),
    };

    // register as a window manager and fail if another WM is running
    if let Err(e) = wm.register() {
        e.handle();
    }

    // setup necessary RandR machinery
    if let Err(e) = wm.init_randr() {
        e.handle();
    }

    // user-defined setup
    setup_wm(&mut wm);

    // find all clients present
    wm.init_clients();

    // main loop
    if let Err(e) = wm.run() {
        e.handle();
    }
}
