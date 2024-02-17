use core::{cell::UnsafeCell, mem::ManuallyDrop};

use collections::{ErasedType, ErasedVec, Ptr};

use super::{
    tracking::{ChangeTracking, TrackingInfo},
    Component, ComponentID, ComponentMetaData,
};

/// Stores type-erased component data for a single component type in a contiguous vec
///
/// # Safety
/// - The underlying data structure that facilitates the type-erasure is an [ErasedVec], which
///   is inherently unsafe due to its use of raw pointers.
/// - Moreover, the responsbility of ensuring that the data inserted into this storage is of the
///   correct type is deferred to the caller for the sake of performance.
/// - The caller must also ensure that any accesses are within the bounds of the underlying vec.
///
/// It is expected that extra bookkeeping is done to ensure that the above invariants are upheld.
pub struct ComponentStorage {
    id: ComponentID,
    components: ErasedVec,
    tracker: Option<ChangeTracking>,
}

impl ComponentStorage {
    pub fn new<C: Component>(id: ComponentID) -> Self {
        Self {
            id,
            components: ErasedVec::new::<C>(),
            tracker: None,
        }
    }

    pub fn from_other(other: &Self) -> Self {
        let erased_type = other.components.erased_type().clone();

        Self {
            id: other.id,
            components: ErasedVec::from_erased_type(erased_type),
            tracker: None,
        }
    }

    pub fn from_metadata(id: ComponentID, metadata: &ComponentMetaData) -> Self {
        let erased_type =
            ErasedType::from_raw_parts(metadata.type_id, metadata.layout, metadata.drop);

        Self {
            id,
            components: ErasedVec::from_erased_type(erased_type),
            tracker: None,
        }
    }

    pub fn id(&self) -> ComponentID {
        self.id
    }

    pub fn len(&self) -> usize {
        self.components.len()
    }

    pub fn enable_tracking(&mut self) {
        if let None = self.tracker {
            self.tracker = Some(ChangeTracking::with_len(self.components.len()));
        }
    }

    pub fn is_tracked(&self) -> bool {
        self.tracker.is_some()
    }

    /// # Safety
    /// - Tracking must be enabled for this component storage.
    pub unsafe fn get_tracker(&self) -> &ChangeTracking {
        debug_assert!(self.is_tracked());
        unsafe { self.tracker.as_ref().unwrap_unchecked() }
    }

    /// # Safety
    /// - Tracking must be enabled for this component storage.
    pub unsafe fn get_mut_tracker(&mut self) -> &mut ChangeTracking {
        debug_assert!(self.is_tracked());
        unsafe { self.tracker.as_mut().unwrap_unchecked() }
    }

    /// # Safety
    /// - The generic type parameter must match the underlying type of this component storage.
    pub unsafe fn push<C: Component>(&mut self, component: C) {
        let mut component = ManuallyDrop::new(component);
        let comp_ptr = Ptr::from(&mut component);

        // SAFETY: Deferred to the caller
        unsafe { self.components.push(comp_ptr) };

        if self.is_tracked() {
            let tracker = self.get_mut_tracker();

            // TODO: we need to get current world tick to update last_write below
            let tick = 0;

            tracker.push(TrackingInfo::new(tick));
            tracker.last_write = tick;
        }
    }

    /// Retrieves a [Ptr] to the component at the given index.
    ///
    /// # Safety
    /// - The index must be within the bounds of the underlying vec.
    pub unsafe fn get_as_ptr(&self, index: usize) -> Ptr {
        debug_assert!(index < self.len());
        // SAFETY: Bounds check deferred to the caller.
        unsafe { self.components.get_unchecked(index) }
    }

    /// # Safety
    /// - The index must be within the bounds of the underlying vec.
    /// - The generic type parameter must match the underlying type of this component storage.
    pub unsafe fn get<C: Component>(&self, index: usize) -> &C {
        debug_assert!(index < self.len());
        // SAFETY: Bounds check deferred to the caller.
        unsafe { self.components.get_unchecked(index).as_ref() }
    }

    /// # Safety
    /// - The index must be within the bounds of the underlying vec.
    /// - The generic type parameter must match the underlying type of this component storage.
    pub unsafe fn get_mut<C: Component>(&mut self, index: usize) -> &mut C {
        debug_assert!(index < self.len());
        // SAFETY: Bounds check deferred to the caller.
        unsafe { self.components.get_unchecked(index).as_mut() }
    }

    /// # Safety
    /// - The index must be within the bounds of the underlying vec.
    pub unsafe fn delete(&mut self, index: usize) {
        debug_assert!(index < self.len());

        // SAFETY: - We are correctly dropping the component
        //         - Deferred bounds check to the caller
        unsafe { self.components.swap_remove_drop_unchecked(index) };

        if self.is_tracked() {
            let tracker = self.get_mut_tracker();
            tracker.delete(index);
        }
    }

    /// # Safety
    /// - The `src_index` must be within the bounds of the underlying source vec.
    /// - The underlying component type of the source and destination component storage must match.
    pub unsafe fn transfer(&mut self, src_index: usize, dst: &mut Self) {
        debug_assert!(src_index < self.len());

        // SAFETY: Bounds and type check deferred to the caller.
        unsafe {
            let ptr = self.components.swap_remove_unchecked(src_index);
            dst.components.push(ptr);
        }

        if self.is_tracked() {
            let tracker = self.get_mut_tracker();
            tracker.delete(src_index);
        }

        if dst.is_tracked() {
            let tracker = dst.get_mut_tracker();

            // TODO: we need to get current world tick to update last_write below
            let tick = 0;

            tracker.push(TrackingInfo::new(tick));
            tracker.last_write = tick;
        }
    }

    /// # Safety
    /// - The generic type parameter must match the underlying type of this component storage.
    #[inline]
    pub unsafe fn as_slice<C: Component>(&self) -> &[C] {
        unsafe { self.components.as_slice::<C>() }
    }

    /// # Safety
    /// - The generic type parameter must match the underlying type of this component storage.
    #[inline]
    pub unsafe fn as_slice_mut<C: Component>(&mut self) -> &mut [C] {
        unsafe { self.components.as_slice_mut::<C>() }
    }

    /// # Safety
    /// - The generic type parameter must match the underlying type of this component storage.
    #[inline]
    pub unsafe fn as_slice_unsafe_cell<C: Component>(&self) -> &[UnsafeCell<C>] {
        unsafe { self.components.as_slice_unsafe_cell::<C>() }
    }
}
