use wm::layout::*;

pub struct Grid {
    max_col: u8,
}

impl Default for Grid {
    fn default() -> Grid {
        Grid {
            max_col: 3,
        }
    }
}

impl Layout for Grid {
    fn arrange(&self, num_windows: usize, screen: &ScreenSize)
        -> Vec<Option<Geometry>> {
        if num_windows > 0 {
            let max_col = if self.max_col > 0 {
                self.max_col
            } else { self.max_col + 1 } as usize;
            let width = screen.width / max_col as u16;
            let height =
                screen.height / (1 + ((num_windows - 1) / max_col)) as u16;
            (0..num_windows)
                .map(|i| {
                    let x = (width + 2) * (i % max_col) as u16;
                    let y = (height + 2) * (i / max_col) as u16;
                    Some(Geometry { x: x, y: y, width: width, height: height })
                })
                .collect()
        } else {
            (0..num_windows).map(|_| None).collect()
        }
    }

    fn right_window(&self, index: usize, max: usize) -> Option<usize> {
        if index != max {
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
        let max_col = if self.max_col > 0 {
            self.max_col
        } else { self.max_col + 1 } as usize;
        if index >= max_col {
            Some(index - max_col)
        } else {
            None
        }
    }

    fn bottom_window(&self, index: usize, max: usize) -> Option<usize> {
        let max_col = if self.max_col > 0 {
            self.max_col
        } else { self.max_col + 1 } as usize;
        if index < max - max_col {
            Some(index + max_col)
        } else {
            None
        }
    }

    fn new_window_as_master(&self) -> bool { false }

    fn edit_layout(&mut self, msg: LayoutMessage) {
        match msg {
            LayoutMessage::MasterFactorAbs(ncol) => self.max_col = ncol,
            LayoutMessage::MasterFactorRel(ncol) =>
                self.max_col = if ncol < 0 {
                    self.max_col.saturating_sub(ncol.abs() as u8)
                } else {
                    self.max_col.saturating_add(ncol.abs() as u8)
                },
            _ => (),
        };
    }
}
