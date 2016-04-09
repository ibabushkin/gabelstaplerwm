pub trait Layout {
    fn arrange(&self, num_windows: usize) -> Vec<Geometry>;
}

pub struct Geometry {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}
