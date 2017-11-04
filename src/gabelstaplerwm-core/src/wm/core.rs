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
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::os::unix::io::AsRawFd;

use libc;

use xcb::base::*;

use wm::config;
use wm::msg::Message;
use wm::tree::Arena;

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
    InputRead(Vec<&'a str>),
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
    pub fn new(fifo: File, con: &Connection) -> CommandInput {
        let buf_fd = setup_pollfd_from_file(&fifo);
        let x_fd = setup_pollfd_from_connection(con);
        let reader = BufReader::new(fifo);

        CommandInput {
            reader,
            buffer: String::new(),
            pollfds: [buf_fd, x_fd],
        }
    }

    /// Get the next input event.
    pub fn get_next(&mut self) -> InputResult {
        if poll(&mut self.pollfds) {
            let buf_fd = self.pollfds[0];
            if buf_fd.revents & libc::POLLIN != 0 {
                self.buffer.clear();

                if let Ok(n) = self.reader.read_line(&mut self.buffer) {
                    if self.buffer.as_bytes()[n - 1] == 0xA {
                        self.buffer.pop();
                    }
                }

                InputResult::InputRead(self.buffer.split_whitespace().collect())
            } else {
                InputResult::XFdReadable
            }
        } else {
            InputResult::PollError
        }
    }
}

/// The core structure handling the X connection and messaging.
///
/// Responsible for handling events from X and messages from the FIFO, as well as to dispatch
/// messages to the appropriate datastructures, and to push the corresponding changes to X.
pub struct WmCore {
    /// The input source to use.
    input: CommandInput,
    /// The screen number the window manager is running on.
    screen_num: i32,
    /// The place where all the internal tree datastructures play.
    arena: Arena,
}

impl WmCore {
    /// Construct a new window manager core object from the necessary parameters.
    pub fn new(fifo: File, con: &Connection, screen_num: i32) -> WmCore {
        WmCore {
            input: CommandInput::new(fifo, con),
            screen_num,
            arena: config::arena_init(Default::default()), // TODO
        }
    }

    /// Run the window manager's main loop, listening to X events and commands from the FIFO.
    pub fn main_loop(&mut self) {
        loop {
            match self.input.get_next() {
                InputResult::InputRead(words) => {
                    if let Some(msg) = Message::parse_from_words(&words) {
                        match_message!(msg, inner_msg => {
                            debug!("received msg: {:?}", inner_msg);
                        });
                    } else {
                        debug!("received words: {:?}", words);
                    }
                },
                InputResult::XFdReadable => {
                    debug!("X event received");
                },
                InputResult::PollError => {
                    debug!("poll(3) returned an error");
                },
            }
        }
    }
}
