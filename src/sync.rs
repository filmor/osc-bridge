use std::time::{Duration, Instant};

type T = f32;
const THRESHOLD: Duration = Duration::from_millis(100);

#[derive(Clone)]
pub struct Sync {
    left: SyncItem,
    right: SyncItem,
    // l2r: Box<dyn Fn(T) -> T>,
    // r2l: Box<dyn Fn(T) -> T>,
}

pub enum Side {
    Left,
    Right,
}

pub use Side::*;

#[derive(Clone, Debug)]
struct SyncItem {
    last_update: Option<Instant>,
    value: T,
}

impl Sync {
    pub fn new() -> Self {
        Sync {
            left: SyncItem::new(),
            right: SyncItem::new(),
        }
    }

    pub fn update(&mut self, side: Side, value: T) -> bool {
        match side {
            Left => do_update(&mut self.left, &self.right, value),
            Right => do_update(&mut self.right, &self.left, value),
        }
    }

    pub fn left_value(&self) -> T {
        self.left.value
    }

    pub fn right_value(&self) -> T {
        self.right.value
    }

    pub fn values(&self) -> (T, T) {
        (self.left.value, self.right.value)
    }
}

impl SyncItem {
    fn new() -> Self {
        SyncItem {
            value: T::default(),
            last_update: None,
        }
    }

    fn update(&mut self, new_value: T) -> bool {
        if self.value != new_value {
            self.value = new_value;
            self.last_update = Some(Instant::now());
            true
        } else {
            false
        }
    }
}

fn do_update(item: &mut SyncItem, other: &SyncItem, value: T) -> bool {
    if let Some(other) = other.last_update {
        if other.elapsed() < THRESHOLD {
            return false;
        }
    }

    item.update(value)
}
