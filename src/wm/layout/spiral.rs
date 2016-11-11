use std::cmp;

use wm::layout::*;

#[derive(Debug)]
pub struct Spiral {
    pub max_windows: u8
}

impl Default for Spiral {
    fn default() -> Spiral {
        Spiral { max_windows: 8 }
    }
}

impl Layout for Spiral {
    fn arrange(&self, num_windows: usize, screen: &TilingArea)
        -> Vec<Option<Geometry>> {
        let mut east = true;
        let mut north = true;
        let mut cur_width = screen.width;
        let mut cur_height = screen.height;
        let mut cur_x = screen.offset_x;
        let mut cur_y = screen.offset_y;

        let min = if num_windows != 0 {
            cmp::min(num_windows, self.max_windows as usize) - 1
        } else { 0 };

        (0..num_windows)
            .map(|i| {
                if num_windows == 1 {
                    // thus, i is 0 as well
                } else if i == 0 {
                    cur_width = cur_width / 2 - 1;
                } else if i > min {
                    return None;
                } else if north && east {
                    if i < min {
                        cur_height = cur_height / 2 - 1;
                    }
                    cur_x += cur_width + 2;
                    north = false;
                } else if !north && east {
                    if i < min {
                        cur_width = cur_width / 2 - 1;
                        cur_x += cur_width + 2;
                    }
                    cur_y += cur_height + 2;
                    east = false;
                } else if !north && !east {
                    if i < min {
                        cur_height = cur_height / 2 - 1;
                        cur_y += cur_height + 2;
                    }
                    cur_x -= cur_width + 2;
                    north = true;
                } else {
                    if i < min {
                        cur_width = cur_width / 2 - 1;
                    }
                    cur_y -= cur_height + 2;
                    east = true;
                }
                Some(Geometry {
                    x: cur_x,
                    y: cur_y,
                    width: cur_width,
                    height: cur_height,
                })
            })
            .collect()
    }

    fn right_window(&self, index: usize, max: usize) -> Option<usize> {
        if index < cmp::max(max, self.max_windows as usize) - 1 {
            Some(index + 1)
        } else {
            None
        }
    }

    fn left_window(&self, index: usize, _: usize) -> Option<usize> {
        if index != 0 {
            Some(index - 1)
        } else {
            None
        }
    }

    fn top_window(&self, index: usize, _: usize) -> Option<usize> {
        if index != 0 {
            Some(index - 1)
        } else {
            None
        }
    }

    fn bottom_window(&self, index: usize, max: usize) -> Option<usize> {
        if index < cmp::max(max, self.max_windows as usize) - 1 {
            Some(index + 1)
        } else {
            None
        }
    }

    fn new_window_as_master(&self) -> bool { false }

    fn edit_layout(&mut self, _: LayoutMessage) -> bool { false }
}
