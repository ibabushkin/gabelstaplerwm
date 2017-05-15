use xcb::xproto::Window;

use wm::layout::Geometry;

#[derive(Clone, Debug)]
pub struct Aien {
    window: Window,
    geometry: Geometry,
}
