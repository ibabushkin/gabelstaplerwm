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

use wm::tree::{ContainerId, Direction, Screen, TagTree, WindowSizes};
use wm::msg::Message;

/// Layout trait.
///
/// Types implementing it can be used to compute window geometries from a tag tree, as well as
/// construct and maintain the tree in a shape suitable for the layout, which we call
/// "layout-consistent".
pub trait Layout {
    /// Compute the geometries to render on a given tagset.
    ///
    /// The input consists of a tagset (and thus the tree of the clients visible on it),
    /// the screen to use, and an output map to be used.
    ///
    /// NB: The tree can be assumed to be in a layout-consistent state.
    /// Geometries output for floating clients are ignored, but rendered using a placeholder
    /// window. The output map can be assumed to be empty.
    fn compute_geo(&self, &TagTree, &Screen, &mut WindowSizes);

    /// Check whether the tree on a tagset is layout-consistent.
    fn check_tree(&self, &TagTree) -> bool;

    /// Transform a given tree in a way that makes it layout-consistent.
    fn correct_tree(&self, &mut TagTree);
    /// Insert a new container into the tree.
    ///
    /// NB: since the container might be essentially an arbitrary subtree, it is not guaranteed
    /// that the tree will be layout-consistent after insertion. This is *allowed*, because a
    /// call to `correct_tree` will be issued from outside.

    fn insert_container(&self, &mut TagTree, ContainerId);
    /// Delete a container from the tree.
    ///
    /// NB: since the container might be essentially an arbitrary subtree, it is not guaranteed
    /// that the tree will be layout-consistent after deletion. This is *allowed*, because a
    /// call to `correct_tree` will be issued from outside.
    fn delete_container(&self, &mut TagTree, ContainerId);

    /// Get a container by direction.
    ///
    /// Used to compute focus and container swapping. In some cases, this transfers the tree in a
    /// non-layout-consistent state. A call to `correct_tree` is then issued.
    fn container_by_direction(&self, &TagTree, ContainerId, Direction) -> Option<ContainerId>;

    /// Accept a message and signifty whether it was accepted.
    ///
    /// Returning `false` implies no change to the layout's state has been performed.
    fn accept_msg(&mut self, Message) -> bool;
}

#[derive(PartialEq, Eq, Clone)]
pub struct Manual;

impl Layout for Manual {
    /// Compute the geometries in a standard fashion.
    fn compute_geo(&self, _: &TagTree, _: &Screen, _: &mut WindowSizes) {
        // TODO: implement
    }

    /// The manual layout considers any tree valid.
    fn check_tree(&self, _: &TagTree) -> bool { true }

    /// No correction is performed, ever.
    fn correct_tree(&self, _: &mut TagTree) { }

    /// Insert a container.
    fn insert_container(&self, _: &mut TagTree, _: ContainerId) {
        // TODO: implement
    }

    /// Remove a container.
    fn delete_container(&self, _: &mut TagTree, _: ContainerId) {
        // TODO: implement
    }

    /// Get a container by direction.
    fn container_by_direction(&self, _: &TagTree, _: ContainerId, _: Direction)
        -> Option<ContainerId>
    {
        // TODO: implement
        None
    }

    /// Drop all messages.
    fn accept_msg(&mut self, _: Message) -> bool {
        false
    }
}

/// The enum holding all possible layouts, and a macro to match on it.
declare_hierarchy!(LayoutContainer; match_layout, Manual);

impl LayoutContainer {
    /// Get a reference to a trait object inside the layout enum.
    ///
    /// This has not much practical use in most cases, but it ensures that all types placed in
    /// variants of the layout enum actually implement the `Layout` trait.
    pub fn as_layout(&self) -> &Layout {
        match_layout!(*self, ref l => l)
    }
}
