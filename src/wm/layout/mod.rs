pub mod grid;
pub mod monocle;
pub mod spiral;
pub mod stack;

/// A screen area size to be accounted for when arranging windows.
///
/// Describes the area used for tiling windows. This allows to leave an open
/// spot for desktop windows, bars and whatever else you might want.
#[derive(Clone, Default)]
pub struct TilingArea {
    /// x offset of tiling area
    pub offset_x: u32,
    /// y offset of tiling area
    pub offset_y: u32,
    /// width of tiling area
    pub width: u32,
    /// height of tiling area
    pub height: u32,
}

impl TilingArea {
    /// Create a new `TilingArea` object.
    ///
    /// Uses a `TilingArea` that represents the user's wishes to get something
    /// that is actually possible.
    pub fn new(old: &TilingArea, width: u32, height: u32) -> TilingArea {
        let new_width = if old.width + old.offset_x < width {
            old.width - old.offset_x
        } else {
            width - old.offset_x
        };
        let new_height = if old.height + old.offset_y < height {
            old.height - old.offset_y
        } else {
            height - old.offset_y
        };
        TilingArea {
            offset_x: old.offset_x,
            offset_y: old.offset_y,
            width: new_width,
            height: new_height,
        }
    }
}

/// A window's geometry.
#[derive(Clone)]
pub struct Geometry {
    /// x coordinate of window
    pub x: u32,
    /// y coordinate of window
    pub y: u32,
    /// width of window
    pub width: u32,
    /// height of window
    pub height: u32,
}

/// Types that compute geometries for arbitrary amounts of windows.
///
/// The only input such objects get are `TilingArea` and number of windows.
pub trait Layout {
    /// Compute window geometries. 
    ///
    /// If a `None` is returned at a particular position, that window is not
    /// to be made visible.
    fn arrange(&self, num_windows: usize, screen: &TilingArea)
        -> Vec<Option<Geometry>>;
    /// Get the window to the right of the nth window.
    fn right_window(&self, index: usize, max: usize) -> Option<usize>;
    /// Get the window to the left of the nth window.
    fn left_window(&self, index: usize, max: usize) -> Option<usize>;
    /// Get the window to the top of the nth window.
    fn top_window(&self, index: usize, max: usize) -> Option<usize>;
    /// Get the window to the bottom of the nth window.
    fn bottom_window(&self, index: usize, max: usize) -> Option<usize>;
    /// Decide whether to insert new windows as master.
    fn new_window_as_master(&self) -> bool;
    /// React to a `LayoutMessage`, returning true on change.
    fn edit_layout(&mut self, msg: LayoutMessage) -> bool;
    /// React to the first applicable `LayoutMessage`, returning true on change.
    fn edit_layout_retry(&mut self, mut msgs: Vec<LayoutMessage>) -> bool {
        msgs.drain(..).any(|m| self.edit_layout(m))
    }

}

/// A message type being sent to layout objects.
///
/// Introduced to allow for type- and implementation-independent layout editing
/// from keybindings and other code. Layout implementations can choose to react
/// to any subset of the message variants below, or none at all.
#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq)]
pub enum LayoutMessage {
    /// Set absolute value of the master factor.
    MasterFactorAbs(u8),
    /// Add an offset to the master factor.
    MasterFactorRel(i8),
    /// Set `fixed` attribute of layout.
    FixedAbs(bool),
    /// Toggle `fixed` attrbute of layout.
    FixedRel,
    /// Set absolute value of the x offset.
    XOffAbs(u32),
    /// Add an offset to the x offset.
    XOffRel(i32),
    /// Set absolute value of the y offset.
    YOffAbs(u32),
    /// Add an offset to the y offset.
    YOffRel(i32),
    /// Set absolute value of the column amount.
    ColumnAbs(u8),
    /// Add an offset to the column amount.
    ColumnRel(i8),
}
