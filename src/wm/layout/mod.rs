pub mod monocle;
pub mod vstack;
pub mod hstack;
pub mod dstack;

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
