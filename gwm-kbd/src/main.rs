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
extern crate gwm_kbd;
#[macro_use]
extern crate log;
extern crate xcb;
extern crate xkb;

use std::env::remove_var;
use std::path::Path;

use xcb::base::*;
use xcb::xkb as xxkb;

use xkb::context::Context;
use xkb::x11 as x11;

use gwm_kbd::kbd::err::{KbdError, KbdResult, XKbdError};
use gwm_kbd::kbd::state::{DaemonState, KbdState};

/// Initialize the logger.
fn setup_logger() {
    // fine to unwrap, as this is the only time we call `init`, so the call will not fail.
    env_logger::init().unwrap();
    info!("initialized logger");

    // clean environment for cargo and other programs honoring `RUST_LOG`
    remove_var("RUST_LOG");
}

/// Main routine.
fn do_main() -> KbdResult<()> {
    setup_logger();

    let (con, screen_num) = match Connection::connect(None) {
        Ok(c) => c,
        Err(_) => {
            return Err(KbdError::X(XKbdError::NoConnection));
        },
    };

    // TODO: own function and error type
    let cookie =
        xxkb::use_extension(&con, x11::MIN_MAJOR_XKB_VERSION, x11::MIN_MINOR_XKB_VERSION);
    match cookie.get_reply() {
        Ok(r) => {
            if !r.supported() {
                return Err(KbdError::X(XKbdError::XKBNotSupported));
            }
        },
        Err(e) => {
            return Err(KbdError::X(XKbdError::NoUseExtensionReply(()))); // TODO
        },
    };

    let core_dev_id = match x11::device(&con) {
        Ok(id) => id,
        Err(()) => return Err(KbdError::X(XKbdError::NoCoreDevice)),
    };
    let context = Context::default();
    let keymap = match x11::keymap(&con, core_dev_id, &context, Default::default()) {
        Ok(k) => k,
        Err(()) => return Err(KbdError::X(XKbdError::CouldNotDetermineKeymap)),
    };
    let state = match x11::state(&con, core_dev_id, &keymap) {
        Ok(s) => s,
        Err(()) => return Err(KbdError::X(XKbdError::CouldNotDetermineState)),
    };

    let map_parts =
        xxkb::MAP_PART_KEY_TYPES |
        xxkb::MAP_PART_KEY_SYMS |
        xxkb::MAP_PART_MODIFIER_MAP |
        xxkb::MAP_PART_EXPLICIT_COMPONENTS |
        xxkb::MAP_PART_KEY_ACTIONS |
        xxkb::MAP_PART_KEY_BEHAVIORS |
        xxkb::MAP_PART_VIRTUAL_MODS |
        xxkb::MAP_PART_VIRTUAL_MOD_MAP;

    let events =
        xxkb::EVENT_TYPE_NEW_KEYBOARD_NOTIFY |
        xxkb::EVENT_TYPE_MAP_NOTIFY |
        xxkb::EVENT_TYPE_STATE_NOTIFY;

    let cookie =
        xxkb::select_events_checked(&con,
                                    xxkb::ID_USE_CORE_KBD as u16,
                                    events as u16,
                                    0,
                                    events as u16,
                                    map_parts as u16,
                                    map_parts as u16,
                                    None);

    cookie.request_check().expect("no events selected");

    let flags =
        xxkb::PER_CLIENT_FLAG_GRABS_USE_XKB_STATE |
        xxkb::PER_CLIENT_FLAG_LOOKUP_STATE_WHEN_GRABBED;

    let cookie =
        xxkb::per_client_flags(&con, xxkb::ID_USE_CORE_KBD as u16, flags, flags, 0, 0, 0);

    cookie.get_reply().expect("no flags set");

    let kbd_state = KbdState::new(&con, screen_num, keymap, state)?;
    let mut daemon_state =
        DaemonState::from_config(Path::new("gwm-kbd/gwmkbdrc.toml"), kbd_state)?;
    debug!("initial daemon state: {:?}", daemon_state);

    daemon_state.grab_current_mode();
    daemon_state.run()
}

fn main() {
    match do_main() {
        Ok(()) => ::std::process::exit(0),
        Err(e) => e.handle(),
    }
}
