use std::time::{Duration, Instant};

type T = f32;
const EPS: f32 = 0.01;

// If the difference is larger than this value, we force a synchronisation
const FORCE_SYNC_EPS: f32 = 1.0;

// How long does the current side count as master?
const MASTER_DURATION: Duration = Duration::from_millis(250);

// Who wins if there is a discrepancy (incomplete update)?
const DEFAULT_MASTER: Side = Side::Right;

#[derive(Clone)]
pub struct Sync {
    name: String,
    left: SyncItem,
    right: SyncItem,
    last_flush: Option<Instant>,
    current_master: Option<Side>,
    // l2r: Box<dyn Fn(T) -> T>,
    // r2l: Box<dyn Fn(T) -> T>,
}

#[derive(Copy, Clone, Debug)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    fn flip(&self) -> Self {
        match self {
            Left => Right,
            Right => Left,
        }
    }
}

pub use Side::*;

#[derive(Clone, Debug)]
struct SyncItem {
    last_update: Option<Instant>,
    value: T,
}

impl Sync {
    pub fn new(name: String) -> Self {
        Sync {
            name,
            left: SyncItem::new(),
            right: SyncItem::new(),
            current_master: None,
            last_flush: None,
        }
    }

    pub fn update(&mut self, side: Side, value: T) -> bool {
        match side {
            Left => self.left.update(value),
            Right => self.right.update(value),
        }
    }

    // pub fn last_update(&self) -> Option<Instant> {
    //     self.left.last_update.max(self.right.last_update)
    // }

    pub fn left_value(&self) -> T {
        self.left.value
    }

    pub fn right_value(&self) -> T {
        self.right.value
    }

    pub fn current_master(&self) -> Option<Side> {
        self.current_master
    }

    pub fn flush(&mut self) -> Option<(T, Side)> {
        let now = Instant::now();
        let threshold = now - MASTER_DURATION;

        let prev_flush = self.last_flush;
        self.last_flush = Some(now);

        match self.current_master {
            // TODO: Handle unsynchronised case! (Every minute from Left -> RIght)
            Some(master) => {
                let item = self.get_item(master);

                if item.last_update < Some(threshold) {
                    // log::info!("Last update: {:?} < threshold {:?}, resetting", item.last_update, threshold);
                    self.current_master = None;
                    return None;
                }

                if item.last_update > prev_flush {
                    // log::info!("Last update: {:?} > prev_flush {:?}, sending {} to {:?}", item.last_update, threshold, item.value, master.flip());
                    return Some((item.value, master.flip()));
                }
            }
            None => {
                let side = if self.left.last_update > self.right.last_update {
                    Left
                } else {
                    Right
                };
                let item = self.get_item(side);
                let value = item.value;

                if item.last_update > prev_flush {
                    self.current_master = Some(side);
                    return Some((value, side.flip()));
                }

                if (self.left.value - self.right.value).abs() > FORCE_SYNC_EPS {
                    let value = self.get_item(side).value;
                    self.current_master = Some(DEFAULT_MASTER);

                    log::warn!(
                        "Force synchronising {} to {:?}: ({}, {}) => {}",
                        self.name,
                        DEFAULT_MASTER,
                        self.left.value,
                        self.right.value,
                        value
                    );
                    return Some((self.get_item(DEFAULT_MASTER).value, DEFAULT_MASTER.flip()));
                }
            }
        }

        None
    }

    fn get_item(&self, side: Side) -> &SyncItem {
        match side {
            Left => &self.left,
            Right => &self.right,
        }
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
        if (self.value - new_value).abs() > EPS {
            self.value = new_value;
            self.last_update = Some(Instant::now());
            true
        } else {
            false
        }
    }
}
