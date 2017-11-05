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

use std::str::FromStr;

use wm::layout::{Layout, LayoutContainer};

/// A message passed to a layout.
///
/// This is constructed in a hierarchic fashion to allow for layouts that don't support all kinds
/// of messages (for example because they don't keep track of master windows).
declare_hierarchy_with_parser!(Message; match_message,
                               (GenericMessage; "generic"),
                               (MasterFactorMessage; "masterf"),
                               (MasterNumberMessage; "nmaster"));

/// A generic message that is interpreted by any layout, by dispatch performed outside of the
/// layout implementation.
#[derive(Debug)]
pub enum GenericMessage {
    // Add a new client on the tag tree managed by the layout.
    // AddClient(ClientId),
}

impl GenericMessage {
    fn parse_from_words(_: &[&str]) -> Option<Self> {
        // TODO: implement
        None
    }
}

/// A message manipulating the master factor of a layout.
///
/// A master factor, if supported by a layout, is a percentage which the layout uses to assign
/// one or more master windows a specific amount of screen space.
#[derive(Debug)]
pub enum MasterFactorMessage {
    /// Set the absolute value of the master factor, saturated to 100.
    Absolute(u8),
    /// Increase the value of the master factor by the given amount, capped to 100.
    Increase(u8),
    /// Decrease the value of the master factor by the given amount, saturated to 0.
    Decrease(u8),
}

impl MasterFactorMessage {
    fn parse_from_words(words: &[&str]) -> Option<Self> {
        if words.len() < 2 {
            return None;
        }

        match words[0] {
            "abs" => u8::from_str(words[1]).ok().map(MasterFactorMessage::Absolute),
            "inc" => u8::from_str(words[1]).ok().map(MasterFactorMessage::Increase),
            "dec" => u8::from_str(words[1]).ok().map(MasterFactorMessage::Decrease),
            _ => None,
        }
    }
}

/// A message manipulating the master number of a layout.
#[derive(Debug)]
pub enum MasterNumberMessage {
    /// Set the absolute value of the master number.
    Absolute(u8),
    /// Increase the value of the master number by the given amount.
    Increase(u8),
    /// Decrease the value of the master number by the given amount, saturated to 1.
    Decrease(u8),
}

impl MasterNumberMessage {
    fn parse_from_words(words: &[&str]) -> Option<Self> {
        if words.len() < 2 {
            return None;
        }

        match words[0] {
            "abs" => u8::from_str(words[1]).ok().map(MasterNumberMessage::Absolute),
            "inc" => u8::from_str(words[1]).ok().map(MasterNumberMessage::Increase),
            "dec" => u8::from_str(words[1]).ok().map(MasterNumberMessage::Decrease),
            _ => None,
        }
    }
}

impl LayoutContainer {
    /// Pass a message to the layout and signify whether it was accepted.
    pub fn accept_msg(&mut self, msg: Message) -> bool {
        match_layout!(*self,
            ref mut l => Layout::accept_msg(l, msg)
        )
    }
}
