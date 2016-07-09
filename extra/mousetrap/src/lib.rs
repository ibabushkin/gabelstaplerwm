pub mod mousetrap {
    //! This module contains utilities to perform systematic mouse warping
    //! based on subsequent reduction of the target area. This allows for a
    //! logarithmic compexity to move the mouse to a specific point.

    /// A direction to halve the target area into.
    pub enum TrapDirection {
        /// Split to the top.
        North,
        /// Split to the right.
        East,
        /// Split to the bottom.
        South,
        /// Split to the left.
        West
    }

    /// A `MouseArena` is the area where the mouse can still be moved to.
    ///
    /// Allows to warp the mouse to the centre of the area (trap it) or
    /// further reduce it's size (close in on it).
    pub struct MouseArena {
        /// Horizontal offset from screen's top left corner of
        /// the current `MouseArena`.
        offset_x: u16,
        /// Vertical offset from screen's top left corner of
        /// the current `MouseArena`.
        offset_y: u16,
        /// Current `MouseArena`'s width.
        width: u16,
        /// Current `MouseArena`'s height.
        height: u16,
        /// minimal width of `MouseArena` allowing for further halving.
        horizontal_min: u8,
        /// minimal height of `MouseArena` allowing for further halving.
        vertical_min: u8,
    }

    impl MouseArena {
        /// Create a new `MouseArena` spanning the entire screen.
        pub fn new(width: u16, height: u16, h_min: u8, v_min: u8)
            -> MouseArena {
            MouseArena {
                offset_x: 0,
                offset_y: 0,
                width: width,
                height: height,
                horizontal_min: h_min,
                vertical_min: v_min,
            }
        }

        /// Reduce the `MouseArea` by a factor of two, given a direction.
        pub fn close_in(&mut self, dir: TrapDirection) -> bool {
            match dir {
                TrapDirection::North =>
                    if self.height >= self.vertical_min as u16{
                        self.height /= 2;
                        true
                    } else {
                        false
                    },
                TrapDirection::East =>
                    if self.width >= self.horizontal_min as u16 {
                        self.width /= 2;
                        true
                    } else {
                        false
                    },
                TrapDirection::South =>
                    if self.height >= self.vertical_min as u16 {
                        self.height /= 2;
                        self.offset_y += self.height;
                        true
                    } else {
                        false
                    },
                TrapDirection::West =>
                    if self.width >= self.horizontal_min as u16 {
                        self.width /= 2;
                        self.offset_x += self.width;
                        true
                    } else {
                        false
                    },
            }
        }

        /// Warp the mouse to the centre of the current `MouseArena`.
        pub fn trap(&self) -> (u16, u16) {
            (self.offset_x + self.width / 2, self.offset_y + self.height / 2)
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {

    }
}
