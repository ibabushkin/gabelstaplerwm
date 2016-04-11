// a screen size to be accounted for when arranging windows
pub struct ScreenSize {
    pub width: u32,
    pub height: u32,
}

// a window's geometry
pub struct Geometry {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

// the layout trait. Types implementing it describe methods to arrange
// windows parametrized over window number and screen size.
// TODO: To be extended to account for dynamic parameters.
pub trait Layout {
    fn arrange(&self, num_windows: usize, screen: &ScreenSize) -> Vec<Geometry>;
}

// the monocle layout with offset
pub struct Monocle {
    pub offset_x: u32,
    pub offset_y: u32,
}

impl Layout for Monocle {
    fn arrange(&self, num_windows: usize, screen: &ScreenSize)
        -> Vec<Geometry> {
        vec![Geometry {x: self.offset_x, y: self.offset_y,
                       width: screen.width - 2 * self.offset_x,
                       height: screen.height - 2 * self.offset_y}
        ]
    }
}

pub fn default_monocle() -> Monocle {
    Monocle {offset_x: 20, offset_y: 20}
}
