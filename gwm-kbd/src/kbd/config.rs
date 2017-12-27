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

use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use toml::value::{Array, Table, Value};

use kbd::err::*;

/// Try to parse a TOML table from a config file, given as a path.
pub fn parse_file(path: &Path) -> KbdResult<Table> {
    match File::open(path) {
        Ok(mut file) => {
            let mut toml_str = String::new();

            match file.read_to_string(&mut toml_str) {
                Ok(_) => {
                    toml_str
                        .parse::<Value>()
                        .map_err(KbdError::TomlError)
                        .and_then(|v| if let Value::Table(t) = v {
                            Ok(t)
                        } else {
                            Err(KbdError::TomlNotTable)
                        })
                },
                Err(io_error) => Err(KbdError::IOError(io_error)),
            }
        },
        Err(io_error) => Err(KbdError::IOError(io_error)),
    }
}

/// Extract a key's value from a table as an int.
pub fn extract_int(table: &mut Table, key: &str) -> KbdResult<i64> {
    match table.remove(key) {
        Some(Value::Integer(i)) => Ok(i),
        Some(_) => Err(KbdError::KeyTypeMismatch(key.to_owned(), false)),
        None => Err(KbdError::KeyMissing(key.to_owned())),
    }
}

/// Extract a key's value from a table as a string.
pub fn extract_string(table: &mut Table, key: &str) -> KbdResult<String> {
    match table.remove(key) {
        Some(Value::String(s)) => Ok(s),
        Some(_) => Err(KbdError::KeyTypeMismatch(key.to_owned(), false)),
        None => Err(KbdError::KeyMissing(key.to_owned())),
    }
}

/// Extract a key's value from a table as a table.
pub fn extract_table(table: &mut Table, key: &str) -> KbdResult<Table> {
    match table.remove(key) {
        Some(Value::Table(t)) => Ok(t),
        Some(_) => Err(KbdError::KeyTypeMismatch(key.to_owned(), false)),
        None => Err(KbdError::KeyMissing(key.to_owned())),
    }
}

/// Extract a key's value from a table as an array.
pub fn extract_array(table: &mut Table, key: &str) -> KbdResult<Array> {
    match table.remove(key) {
        Some(Value::Array(a)) => Ok(a),
        Some(_) => Err(KbdError::KeyTypeMismatch(key.to_owned(), false)),
        None => Err(KbdError::KeyMissing(key.to_owned())),
    }
}

/// Check for an optional key to extract.
pub fn opt_key<T>(input_result: KbdResult<T>) -> KbdResult<Option<T>> {
    match input_result {
        Ok(res) => Ok(Some(res)),
        Err(KbdError::KeyMissing(_)) => Ok(None),
        Err(err) => Err(err),
    }
}
