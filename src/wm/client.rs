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

use std::collections::{HashMap, HashSet};

use xcb::xproto;

use wm::config::Tag;
use wm::layout::Layout;

/// A rectangle somewhere on screen.
///
/// Could represent a client's geometry, a screen, or something else.
pub struct Geometry {
    /// The x coordinate of the upper left corner of the rectangle.
    x: u32,
    /// The y coordinate of the upper left corner of the rectangle.
    y: u32,
    /// The width of the rectangle.
    width: u32,
    /// The height of the rectangle.
    height: u32,
}

/// A unique identifier for clients, in this case provided by the X server.
pub type ClientId = xproto::Window;

/// A client being managed.
pub struct Client {
    /// The client's window (also used as an id).
    window: ClientId,
    /// The client's last-configured geometry.
    geometry: Geometry,
    /// Whether the client's window is currently mapped on screen.
    mapped: bool,
    /// Properties of the client (currently empty).
    properties: (),
    /// The set of tags attached to the client.
    tags: HashSet<Tag>,
}

/// A unique identifier for tagsets, provided by the arena.
pub type TagSetId = u16;

/// A tagset.
///
/// A tagset (written without spaces) is a set of tags, an associated tag tree, and a pointer to
/// a layout. This means there can be multiple tagsets with the same set of tags at any given
/// point in time.
pub struct TagSet {
    /// The set of tags wrapped by the tagset.
    tags: HashSet<Tag>,
    /// The tag tree maintained by the layout on the given tagset.
    tree: TagTree,
    /// The layout used.
    layout: Box<Layout>,
}

/// A unique identifier for screens, provided by the arena.
pub type ScreenId = u8;

/// A screen showing a tagset.
pub struct Screen {
    /// The tagset currently shown.
    tagset: TagSetId,
    /// The screen's geometry.
    geometry: Geometry,
}

/// A split type used in a container.
pub enum SplitType {
    /// A horizontal split.
    Horizontal,
    /// A vertical split.
    Vertical,
    /// A tabbed "split".
    ///
    /// This is basically a container that allows to tab through multiple clients.
    Tabbed,
}

/// A unique indentifier for containers, provided by the tag tree they are located in.
pub type ContainerId = u16;

/// A container representing an inner node in a tag tree.
pub struct SplitContainer {
    /// The split type of the container.
    split_type: SplitType,
    /// The child container last focused.
    last_focused: Option<u16>,
    /// The ordered set of child containers.
    children: Vec<u16>,
    /// Whether the container is marked floating.
    floating: bool,
}

/// A container representing a leaf in a tag tree.
pub struct ClientContainer {
    /// The client in the container.
    client: ClientId,
    /// Whether the container is marked floating.
    floating: bool,
}

/// A container representing an arbitrary node in a tag tree.
pub enum Container {
    /// A split container.
    SplitContainer(SplitContainer),
    /// A client container.
    ClientContainer(ClientContainer),
}

/// A tag tree.
///
/// Represents a rose tree of containers with all the windows visible on a set of tags,
/// structured according to a layout. Always contains a root node.
pub struct TagTree {
    /// An arena of containers.
    container_arena: Vec<Container>,
    /// The root container of the tree.
    root: ContainerId,
    /// The focused container in the tree.
    focused: Option<ContainerId>,
    /// The selected container in the tree.
    selected: Option<ContainerId>,
}

/// The toplevel structure keeping track of clients, tagsets, and screens.
pub struct Arena {
    /// The set of clients.
    clients: HashMap<ClientId, Client>,
    /// The set of tagsets, indexed by values of type `TagSetId`.
    tagsets: Vec<TagSet>,
    /// The set of screens, indexed by values of type `ScreenId`.
    screens: Vec<Screen>,
}
