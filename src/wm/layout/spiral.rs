use wm::layout::*;

pub struct Spiral { }

impl Default for Spiral {
    fn default() -> Spiral {
        Spiral { }
    }
}

impl Layout for Spiral {
    fn arrange(&self, num_windows: usize, screen: &ScreenSize)
        -> Vec<Option<Geometry>> {
        let mut east = true;
        let mut north = true;
        let mut cur_width = screen.width;
        let mut cur_height = screen.height;
        let mut cur_x = screen.offset_x;
        let mut cur_y = screen.offset_y;

        if num_windows == 1 {
            vec![Some(Geometry {
                x: cur_x,
                y: cur_y,
                width: cur_width,
                height: cur_height
            })]
        } else {
            (0..num_windows)
                .map(|i| {
                    if i == 0 {
                        cur_width = cur_width / 2 - 2;
                    } else if north && east {
                        if i != num_windows - 1 {
                            cur_height = cur_height / 2 - 2;
                        }
                        cur_x += cur_width + 2;
                        north = false;
                    } else if !north && east {
                        if i != num_windows - 1 {
                            cur_width = cur_width / 2 - 2;
                            cur_x += cur_width + 2;
                        }
                        cur_y += cur_height + 2;
                        east = false;
                    } else if !north && !east {
                        if i != num_windows - 1 {
                            cur_height = cur_height / 2 - 2;
                            cur_y += cur_height + 2;
                        }
                        cur_x -= cur_width + 2;
                        north = true;
                    } else {
                        if i != num_windows - 1 {
                            cur_width = cur_width / 2 - 2;
                            cur_x += cur_width + 2;
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
    }

    fn right_window(&self, index: usize, max: usize) -> Option<usize> {
        None
    }

    fn left_window(&self, index: usize, max: usize) -> Option<usize> {
        None
    }

    fn top_window(&self, index: usize, _: usize) -> Option<usize> {
        if index != 0 {
            Some(index - 1)
        } else {
            None
        }
    }

    fn bottom_window(&self, index: usize, max: usize) -> Option<usize> {
        if index != max {
            Some(index + 1)
        } else {
            None
        }
    }

    fn new_window_as_master(&self) -> bool { false }

    fn edit_layout(&mut self, _: LayoutMessage) { }
}
