/*
 * Copyright Inokentiy Babushkin and contributors (c) 2016-2017
 *
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 *
 *     * Redistributions of source code must retain the above copyright
 *       notice, this list of conditions and the following disclaimer.
 *
 *     * Redistributions in binary form must reproduce the above
 *       copyright notice, this list of conditions and the following
 *       disclaimer in the documentation and/or other materials provided
 *       with the distribution.
 *
 *     * Neither the name of Inokentiy Babushkin nor the names of other
 *       contributors may be used to endorse or promote products derived
 *       from this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
 * "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
 * LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
 * A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
 * OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
 * SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
 * LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
 * OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */

extern crate env_logger;
extern crate getopts;
extern crate gwm_kbd;
#[macro_use]
extern crate log;
extern crate xcb;
extern crate xkb;

use getopts::Options;

use std::env;
use std::mem;
use std::path::{Path, PathBuf};

use xcb::base::*;
use xcb::ffi::xkb as xxkb_ffi;
use xcb::xkb as xxkb;

use xkb::context::Context;
use xkb::x11 as x11;

use gwm_kbd::kbd::err::{KbdError, KbdResult, XError};
use gwm_kbd::kbd::state::{DaemonState, KbdState};

/// Initialize the logger.
fn setup_logger() {
    env_logger::init();
    info!("initialized logger");

    // clean environment for cargo and other programs honoring `RUST_LOG`
    env::remove_var("RUST_LOG");
}

/// Main routine.
fn do_main(path: &Path) -> KbdResult<()> {
    let (con, screen_num) = match Connection::connect(None) {
        Ok(c) => c,
        Err(e) => {
            return Err(XError::CouldNotConnect(e).wrap());
        },
    };

    let cookie =
        xxkb::use_extension(&con, x11::MIN_MAJOR_XKB_VERSION, x11::MIN_MINOR_XKB_VERSION);
    match cookie.get_reply() {
        Ok(r) => {
            if !r.supported() {
                return Err(XError::XKBNotSupported.wrap());
            }
        },
        Err(e) => {
            return Err(XError::UseExtensionError(e).wrap());
        },
    };

    let core_dev_id = match x11::device(&con) {
        Ok(id) => id,
        Err(()) => return Err(XError::CouldNotDetermineCoreDevice.wrap()),
    };
    let context = Context::default();
    let keymap = match x11::keymap(&con, core_dev_id, &context, Default::default()) {
        Ok(k) => k,
        Err(()) => return Err(XError::CouldNotDetermineKeymap.wrap()),
    };

    let state = match x11::state(&con, core_dev_id, &keymap) {
        Ok(s) => s,
        Err(()) => return Err(XError::CouldNotDetermineState.wrap()),
    };

    let events =
        (xxkb::EVENT_TYPE_NEW_KEYBOARD_NOTIFY |
         xxkb::EVENT_TYPE_MAP_NOTIFY |
         xxkb::EVENT_TYPE_STATE_NOTIFY) as u16;

    let nkn_details = xxkb::NKN_DETAIL_KEYCODES as u16;

    let map_parts =
        (xxkb::MAP_PART_KEY_TYPES |
         xxkb::MAP_PART_KEY_SYMS |
         xxkb::MAP_PART_MODIFIER_MAP |
         xxkb::MAP_PART_EXPLICIT_COMPONENTS |
         xxkb::MAP_PART_KEY_ACTIONS |
         // xxkb::MAP_PART_KEY_BEHAVIORS |
         xxkb::MAP_PART_VIRTUAL_MODS |
         xxkb::MAP_PART_VIRTUAL_MOD_MAP) as u16;

    let state_details =
        (xxkb::STATE_PART_MODIFIER_BASE |
         xxkb::STATE_PART_MODIFIER_LATCH |
         xxkb::STATE_PART_MODIFIER_LOCK |
         xxkb::STATE_PART_GROUP_BASE |
         xxkb::STATE_PART_GROUP_LATCH |
         xxkb::STATE_PART_GROUP_LOCK) as u16;

    let mut details: xxkb_ffi::xcb_xkb_select_events_details_t = unsafe { mem::zeroed() };
    details.affectNewKeyboard = nkn_details;
    details.newKeyboardDetails = nkn_details;
    details.affectState = state_details;
    details.stateDetails = state_details;

    let cookie = unsafe {
        let c = xxkb_ffi::xcb_xkb_select_events_checked(
            con.get_raw_conn(),
            xxkb::ID_USE_CORE_KBD as xxkb_ffi::xcb_xkb_device_spec_t, /* device_spec */
            events as u16, /* affect_which */
            0, /* clear */ 0, /* select_all */
            map_parts as u16, /* affect_map */
            map_parts as u16, /* map */
            &details as *const xxkb_ffi::xcb_xkb_select_events_details_t);

        VoidCookie {
            cookie: c,
            conn: &con,
            checked: true,
        }
    };

    // TODO: proper error handling
    cookie.request_check().expect("no events selected");

    let flags =
        xxkb::PER_CLIENT_FLAG_GRABS_USE_XKB_STATE |
        xxkb::PER_CLIENT_FLAG_LOOKUP_STATE_WHEN_GRABBED;

    let cookie =
        xxkb::per_client_flags(&con, xxkb::ID_USE_CORE_KBD as u16, flags, flags, 0, 0, 0);

    // TODO: proper error handling
    cookie.get_reply().expect("no flags set");

    let kbd_state = KbdState::new(&con, screen_num, keymap, state)?;
    let mut daemon_state =
        DaemonState::from_config(path, kbd_state)?;
    debug!("initial daemon state: {:?}", daemon_state);

    daemon_state.grab_current_mode();
    daemon_state.run()
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // set up option parsing
    let mut opts = Options::new();
    opts.optopt("c", "config", "set config file name", "FILE");
    opts.optflag("h", "help", "print this help menu");

    // match on args and decide what to do
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => KbdError::CouldNotParseOptions(f).handle(),
    };

    if matches.opt_present("h") {
        let brief = format!("Usage: {} [options]", &args[0]);
        eprintln!("{}", opts.usage(&brief));
        return;
    }

    let config_path = if let Some(p) = matches.opt_str("c") {
        p.into()
    } else if let Some(mut buf) = env::home_dir() {
        buf.push(".gwmkbdrc");
        buf
    } else {
        warn!("couldn't determine the value of $HOME, using current dir");
        PathBuf::from("gwmkbdrc")
    };

    setup_logger();

    match do_main(&config_path) {
        Ok(()) => ::std::process::exit(0),
        Err(e) => e.handle(),
    }
}
