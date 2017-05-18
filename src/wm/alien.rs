use xcb::xproto::Window;

use wm::layout::Geometry;

#[derive(Clone, Debug)]
pub struct Alien {
    window: Window,
    geometry: Geometry,
}

impl Alien {
    pub fn new(window: Window, geometry: Geometry) -> Alien {
        Alien {
            window: window,
            geometry: geometry,
        }
    }

    pub fn get_window(&self) -> &Window {
        &self.window
    }
}
