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

use std::io::Error as IoError;

use toml;

/// An error occured when reading in the configuration.
#[derive(Debug)]
pub enum KbdError {
    /// An I/O error occured.
    IOError(IoError),
    /// The TOML content of the config file is invalid.
    TomlError(toml::de::Error),
    /// The TOML file does not contain a toplevel table.
    TomlNotTable,
    /// A necessary config key is missing.
    KeyMissing(String),
    /// A config key holds a value of the wrong type. Second field set to true if it's a command
    /// key.
    KeyTypeMismatch(String, bool),
    /// A Keysym could not be parsed.
    KeysymCouldNotBeParsed(String),
    /// An invalid chord has been passed into the config.
    InvalidChord,
}

impl KbdError {
    pub fn handle(self) -> ! {
        use kbd::err::KbdError::*;

        match self {
            IOError(i) => error!("I/O error occured: {}", i),
            TomlError(t) => error!("TOML parsing of config failed: {}", t),
            TomlNotTable => error!("config is not a table at the top level"),
            KeyMissing(k) => error!("missing config key: {}", k),
            KeyTypeMismatch(k, false) => error!("key {} has incorrect type", k),
            KeyTypeMismatch(k, true) => error!("command bound to `{}` has non-string type", k),
            KeysymCouldNotBeParsed(k) => error!("could not parse keysym: {}", k),
            InvalidChord => error!("chord invalid: {}", "<placeholder>"),
        }

        ::std::process::exit(1);
    }
}

/// A result returned when reading in the configuration.
pub type KbdResult<T> = Result<T, KbdError>;
