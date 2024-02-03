
#[derive(Default, Clone)]
pub struct TrackingInfo {
    pub modified: u32,
}

impl TrackingInfo {
    pub fn new(modified: u32) -> Self {
        Self { modified }
    }
}

pub struct ChangeTracking {
    pub info: Vec<TrackingInfo>,
    pub last_read: u32,
    pub last_write: u32,
}

impl ChangeTracking {
    pub fn new() -> Self {
        Self {
            info: Vec::new(),
            last_read: 0,
            last_write: 0,
        }
    }

    pub fn with_len(len: usize) -> Self {
        Self {
            info: vec![TrackingInfo::default(); len],
            last_read: 0,
            last_write: 0,
        }
    }

    pub fn push(&mut self) {
        self.info.push(TrackingInfo::new(self.last_write));
    }

    /// # Safety
    /// - The index must be within the bounds of the underlying vec.
    pub unsafe fn get(&self, index: usize) -> &TrackingInfo {
        debug_assert!(index < self.info.len());
        unsafe { self.info.get_unchecked(index) }
    }

    /// # Safety
    /// - The index must be within the bounds of the underlying vec.
    pub unsafe fn get_mut(&mut self, index: usize) -> &mut TrackingInfo {
        debug_assert!(index < self.info.len());
        unsafe { self.info.get_unchecked_mut(index) }
    }

    /// # Safety
    /// - The index must be within the bounds of the underlying vec.
    pub unsafe fn delete(&mut self, index: usize) {
        debug_assert!(index < self.info.len());
        self.info.swap_remove(index);
    }
}
