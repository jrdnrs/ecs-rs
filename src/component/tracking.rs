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
    /// The length of this will always match the length of the component storage's vec.
    /// It stores the world tick at which various things occurred to the component.
    info: Vec<TrackingInfo>,

    /// This is updated after a system has run for a set of components.
    ///
    /// When a system runs for a set of components, we cannot guarantee whether each one has been read, so
    /// we store the read tick for the entire set. But, we *can* know when a component has been written to,
    /// with user submitted commands, so we store the write tick for each component and compare them to
    /// detect changes.
    pub(crate) last_read: u32,

    /// This is updated whenever a new component is added, or when the user issues a `FlagModifiedCommand`
    /// for a component.
    ///
    /// This is the tick of when the last modification to a component occurred.
    pub(crate) last_write: u32,
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

    pub fn push(&mut self, info: TrackingInfo) {
        self.info.push(info);
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

    /// # Panics
    /// Panics if the index is out of bounds.
    pub fn delete(&mut self, index: usize) {
        self.info.swap_remove(index);
    }
}
