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
pub trait Layout {
    // compute window geometries
    fn arrange(&self, num_windows: usize, screen: &ScreenSize)
        -> Vec<Option<Geometry>>;
    // get the window to the right of the nth window
    fn right_window(&self, index: usize, max: usize) -> Option<usize>;
    // get the window to the left of the nth window
    fn left_window(&self, index: usize, max: usize) -> Option<usize>;
    // get the window to the top of the nth window
    fn top_window(&self, index: usize, max: usize) -> Option<usize>;
    // get the window to the bottom of the nth window
    fn bottom_window(&self, index: usize, max: usize) -> Option<usize>;
}