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
extern crate gabelstaplerwm;
extern crate libc;
#[macro_use]
extern crate log;
extern crate xcb;

use std::env::remove_var;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::ptr::null_mut;
use std::os::unix::io::AsRawFd;
use std::str::SplitWhitespace;

use gabelstaplerwm::wm::err::WmError;

use xcb::base::*;

/// Reap children.
extern "C" fn sigchld_action(_: libc::c_int) {
    while unsafe { libc::waitpid(-1, null_mut(), libc::WNOHANG) } > 0 { }
}

/// Construct a `pollfd` struct from a file reference.
fn setup_pollfd_from_file(fd: &File) -> libc::pollfd {
    libc::pollfd {
        fd: fd.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    }
}

/// Construct a `pollfd` struct from a raw file descriptor.
fn setup_pollfd_from_connection(con: &Connection) -> libc::pollfd {
    libc::pollfd {
        fd: con.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    }
}

/// `poll(3)` a slice of `pollfd` structs and tell us whether everything went well.
fn poll(fds: &mut [libc::pollfd]) -> bool {
    let poll_res = unsafe {
        libc::poll(fds.as_mut_ptr(), fds.len() as u64, -1)
    };

    poll_res > 0
}

/// The possible input events we get from a command input handler.
pub enum InputResult<'a> {
    /// The words handed down by the iterator have been read from the input pipe.
    InputRead(SplitWhitespace<'a>),
    /// The X connection's socket has some data.
    XFdReadable,
    /// Poll returned an error.
    PollError,
}

/// The command input handler.
pub struct CommandInput {
    /// The buffered reader for the input pipe.
    reader: BufReader<File>,
    /// The buffer to use for reading.
    buffer: String,
    /// The `pollfd` structs polled by the command input handler.
    ///
    /// The first entry is the input pipe, the socond is the X connection socket.
    pollfds: [libc::pollfd; 2],
}

impl CommandInput {
    /// Construct an input handler from a file representing the input pipe and an X connection.
    pub fn new(file: File, con: &xcb::Connection) -> CommandInput {
        let buf_fd = setup_pollfd_from_file(&file);
        let x_fd = setup_pollfd_from_connection(con);
        let reader = BufReader::new(file);

        CommandInput {
            reader,
            buffer: String::new(),
            pollfds: [buf_fd, x_fd],
        }
    }

    /// Get the next input event.
    pub fn get_next_input(&mut self) -> InputResult {
        if poll(&mut self.pollfds) {
            let buf_fd = self.pollfds[0];
            if buf_fd.revents & libc::POLLIN != 0 {
                self.buffer.clear();

                if let Ok(n) = self.reader.read_line(&mut self.buffer) {
                    if self.buffer.as_bytes()[n - 1] == 0xA {
                        self.buffer.pop();
                    }
                }

                InputResult::InputRead(self.buffer.split_whitespace())
            } else {
                InputResult::XFdReadable
            }
        } else {
            InputResult::PollError
        }
    }
}

/// Main function.
fn main() {
    // fine to unwrap, as this is the only time we call `init`.
    env_logger::init().unwrap();
    info!("initialized logger");

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

    // clean environment for cargo and other programs honoring `RUST_LOG`
    remove_var("RUST_LOG");

    let (con, screen_num) = match Connection::connect(None) {
        Ok(c) => c,
        Err(e) => {
            WmError::CouldNotConnect(e).handle();
        },
    };
}
