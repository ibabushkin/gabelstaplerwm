/*
 * Copyright Inokentiy Babushkin and contributors (c) 2016-2017
 *
 * All rights reserved.

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
extern crate gwm_core as gabelstaplerwm;
extern crate getopts;
extern crate libc;
#[macro_use]
extern crate log;
extern crate xcb;

use getopts::Options;

use std::env::{args, home_dir, remove_var};
use std::ffi::CString;
use std::fs::{File, OpenOptions};
use std::os::unix::fs::FileTypeExt;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::ptr::null_mut;

use gabelstaplerwm::wm::core::WmCore;
use gabelstaplerwm::wm::err::WmError;

use xcb::base::*;

/// Reap children.
extern "C" fn sigchld_action(_: libc::c_int) {
    while unsafe { libc::waitpid(-1, null_mut(), libc::WNOHANG) } > 0 { }
}

/// Initialize the logger and unset the `RUST_LOG` environment variable afterwards.
fn setup_logger() {
    // fine to unwrap, as this is the only time we call `init`.
    env_logger::init().unwrap();
    info!("initialized logger");

    // clean environment for cargo and other programs honoring `RUST_LOG`
    remove_var("RUST_LOG");
}

/// Set up signal handling for `SIGCHLD`.
fn setup_sigaction() {
    // we're a good parent - we wait for our children when they get a screaming
    // fit at the checkout lane
    unsafe {
        use std::mem;

        // initialize the sigaction struct
        let mut act = mem::uninitialized::<libc::sigaction>();

        // convert our handler to a C-style function pointer
        let f_ptr: *const libc::c_void =
            mem::transmute(sigchld_action as extern "C" fn(libc::c_int));
        act.sa_sigaction = f_ptr as libc::sighandler_t;

        // some default values noone cares about
        libc::sigemptyset(&mut act.sa_mask);
        act.sa_flags = libc::SA_RESTART;

        // setup our SIGCHLD-handler
        if libc::sigaction(libc::SIGCHLD, &act, null_mut()) == -1 {
            // crash and burn on failure
            WmError::CouldNotEstablishSignalHandlers.handle();
        }
    }
}

/// Set up a FIFO at the given path.
fn setup_fifo(path: &Path) -> File {
    let mut options = OpenOptions::new();
    options.read(true);
    options.write(true);

    match options.open(path) {
        Ok(fifo) => {
            match fifo.metadata().map(|m| m.file_type().is_fifo()) {
                Ok(true) => fifo,
                _ => WmError::CouldNotOpenPipe.handle(),
            }
        }
        _ => {
            let path_cstr = CString::new(path.as_os_str().as_bytes()).unwrap();
            let perms = libc::S_IRUSR | libc::S_IWUSR;
            let ret = unsafe { libc::mkfifo(path_cstr.as_ptr() as *const i8, perms) };
            if ret != 0 {
                WmError::CouldNotOpenPipe.handle()
            } else {
                options.open(path).ok().unwrap_or_else(|| WmError::CouldNotOpenPipe.handle())
            }
        },
    }
}

/// Determine the path to use for the input FIFO.
fn setup_fifo_path() -> PathBuf {
    if let Some(mut buf) = home_dir() {
        buf.push("tmp");
        buf.push("gwm_fifo");
        buf
    } else {
        warn!("couldn't determine the value of $HOME, using current dir");
        PathBuf::from("gwm_fifo")
    }
}

/// Main function.
fn main() {
    setup_logger();

    let args: Vec<String> = args().collect();

    let mut opts = Options::new();
    opts.optopt("f", "fifo", "input pipe to use", "FIFO");
    opts.optflag("h", "help", "print this help menu");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => {
            WmError::CouldNotParseOptions(e).handle();
        },
    };

    if matches.opt_present("h") {
        let brief = format!("Usage: {} [options]", &args[0]);
        eprintln!("{}", opts.usage(&brief));
        return;
    }

    let fifo = if let Some(p) = matches.opt_str("f") {
        setup_fifo(Path::new(&p))
    } else {
        let path = setup_fifo_path();
        setup_fifo(&path)
    };

    let (con, screen_num) = match Connection::connect(None) {
        Ok(c) => c,
        Err(e) => {
            WmError::CouldNotConnect(e).handle();
        },
    };

    setup_sigaction();

    let mut core = WmCore::new(fifo, &con, screen_num);

    core.main_loop();
}
