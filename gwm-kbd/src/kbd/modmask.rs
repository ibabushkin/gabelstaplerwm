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

use xcb::xproto;

use xkb;

/// Update a given modifier mask.
pub fn combine(mask: &mut xkb::ModMask, add_mask: xkb::ModMask) {
    use xcb::ffi::xcb_mod_mask_t;

    *mask = xkb::ModMask(mask.0 as xcb_mod_mask_t | add_mask.0 as xcb_mod_mask_t);
}

const LOCK_MASK: xkb::ModMask=  xkb::ModMask(xproto::MOD_MASK_LOCK);
const NUM_MASK: xkb::ModMask = xkb::ModMask(xproto::MOD_MASK_2);
const IGNORE_MASK: xkb::ModMask = xkb::ModMask(xproto::MOD_MASK_LOCK | xproto::MOD_MASK_2);

/// Filter ignored modifiers from a mask
pub fn filter_ignore(mask: &mut xkb::ModMask) {
    use xcb::ffi::xcb_mod_mask_t;

    *mask = xkb::ModMask(mask.0 as xcb_mod_mask_t & !IGNORE_MASK.0);
}

/// Construct a set of modifier masks to grab for a keybinding to account for ignored modifiers.
pub fn match_ignore(mask: xkb::ModMask) -> [xkb::ModMask; 4] {
    let mut res = [mask, mask, mask, mask];

    combine(&mut res[1], LOCK_MASK);
    combine(&mut res[2], NUM_MASK);
    combine(&mut res[3], IGNORE_MASK);

    res
}

/// Get a modifier mask from a string description of the modifier keys.
pub fn from_str(desc: &str, mask: &mut xkb::ModMask) -> bool {
    let mut mod_component: xkb::ModMask = xkb::ModMask(match &desc.to_lowercase()[..] {
        "shift" => xproto::MOD_MASK_SHIFT,
        "ctrl" => xproto::MOD_MASK_CONTROL,
        "mod1" => xproto::MOD_MASK_1,
        "mod2" => xproto::MOD_MASK_2,
        "mod3" => xproto::MOD_MASK_3,
        "mod4" => xproto::MOD_MASK_4,
        "mod5" => xproto::MOD_MASK_5,
        _ => 0,
    });

    filter_ignore(&mut mod_component);
    combine(mask, mod_component);

    mod_component.0 != 0
}
