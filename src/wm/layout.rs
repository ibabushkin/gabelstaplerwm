// a screen size to be accounted for when arranging windows
pub struct ScreenSize {
    pub width: u16,
    pub height: u16,
}

// a window's geometry
pub struct Geometry {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

// the layout trait. Types implementing it describe methods to arrange
// windows parametrized over window number and screen size.
// TODO: To be extended to account for dynamic parameters.
pub trait Layout {
    fn arrange(&self, num_windows: usize, screen: &ScreenSize)
        -> Vec<Option<Geometry>>;
}

// the monocle layout with offset
pub struct Monocle {
    pub offset_x: u16,
    pub offset_y: u16,
}

impl Layout for Monocle {
    fn arrange(&self, num_windows: usize, screen: &ScreenSize)
        -> Vec<Option<Geometry>> {
        let mut res = Vec::with_capacity(num_windows);
        res.push(Some(Geometry {x: self.offset_x, y: self.offset_y,
            width: screen.width - 2 * self.offset_x,
            height: screen.height - 2 * self.offset_y}));
        for _ in 1..num_windows {
            res.push(None);
        }
        res
    }
}

impl Monocle { 
    pub fn default() -> Monocle {
        Monocle {offset_x: 20, offset_y: 20}
    }
}
