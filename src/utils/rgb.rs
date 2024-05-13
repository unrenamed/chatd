use fnv::FnvHasher;
use std::hash::{Hash, Hasher};

pub fn to_rgb(s: &str) -> (u8, u8, u8) {
    let mut hasher = FnvHasher::default();
    s.hash(&mut hasher);
    let hash = hasher.finish();

    let r = (hash & 0xFF) as u8;
    let g = ((hash >> 8) & 0xFF) as u8;
    let b = ((hash >> 16) & 0xFF) as u8;
    (r, g, b)
}
