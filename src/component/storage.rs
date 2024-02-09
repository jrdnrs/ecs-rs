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
    pub id: ComponentID,
    pub components: ErasedVec,
    pub tracker: Option<ChangeTracking>,
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

    pub fn get_tracker(&self) -> Option<&ChangeTracking> {
        self.tracker.as_ref()
    }

    pub fn get_tracker_mut(&mut self) -> Option<&mut ChangeTracking> {
        self.tracker.as_mut()
    }

    /// # Safety
    /// - Tracking must be enabled for this component storage.
    pub unsafe fn get_tracker_unchecked(&self) -> &ChangeTracking {
        debug_assert!(self.is_tracked());
        unsafe { self.tracker.as_ref().unwrap_unchecked() }
    }

    /// # Safety
    /// - Tracking must be enabled for this component storage.
    pub unsafe fn get_tracker_mut_unchecked(&mut self) -> &mut ChangeTracking {
        debug_assert!(self.is_tracked());
        unsafe { self.tracker.as_mut().unwrap_unchecked() }
    }

    pub fn get_tracking_info(&self, index: usize) -> Option<&TrackingInfo> {
        self.tracker.as_ref().and_then(|t| t.get(index))
    }

    pub fn get_tracking_info_mut(&mut self, index: usize) -> Option<&mut TrackingInfo> {
        self.tracker.as_mut().and_then(|t| t.get_mut(index))
    }

    /// # Safety
    /// - The index must be within the bounds of the underlying vec.
    /// - Tracking must be enabled for this component storage.
    pub unsafe fn get_tracking_info_unchecked(&self, index: usize) -> &TrackingInfo {
        unsafe { self.get_tracker_unchecked().get_unchecked(index) }
    }

    /// # Safety
    /// - The index must be within the bounds of the underlying vec.
    /// - Tracking must be enabled for this component storage.
    pub unsafe fn get_tracking_info_mut_unchecked(&mut self, index: usize) -> &mut TrackingInfo {
        unsafe { self.get_tracker_mut_unchecked().get_unchecked_mut(index) }
    }

    /// # Safety
    /// - The generic type parameter must match the underlying type of this component storage.
    pub unsafe fn push<C: Component>(&mut self, component: C) {
        let component = ManuallyDrop::new(component);
        let comp_ptr = Ptr::from(&component as *const _ as *mut u8);

        // SAFETY: Deferred to the caller
        unsafe { self.components.push(comp_ptr) };

        if self.is_tracked() {
            let tracker = self.get_tracker_mut_unchecked();

            // TODO: we need to get current world tick to update last_write below
            let tick = 0;

            tracker.push(TrackingInfo::new(tick));
            tracker.last_write = tick;
        }
    }

    pub unsafe fn get_as_ptr(&self, index: usize) -> Option<Ptr> {
        // SAFETY: Bounds check deferred to the caller.
        unsafe { self.components.get(index) }
    }

    /// Retrieves a [Ptr] to the component at the given index.
    ///
    /// # Safety
    /// - The index must be within the bounds of the underlying vec.
    pub unsafe fn get_as_ptr_unchecked(&self, index: usize) -> Ptr {
        debug_assert!(index < self.len());
        // SAFETY: Bounds check deferred to the caller.
        unsafe { self.components.get_unchecked(index) }
    }

    /// # Safety
    /// - The generic type parameter must match the underlying type of this component storage.
    pub unsafe fn get<C: Component>(&self, index: usize) -> Option<&C> {
        // SAFETY: Bounds check deferred to the caller.
        unsafe { self.components.get(index).map(|ptr| ptr.as_ref()) }
    }

    /// # Safety
    /// - The index must be within the bounds of the underlying vec.
    /// - The generic type parameter must match the underlying type of this component storage.
    pub unsafe fn get_unchecked<C: Component>(&self, index: usize) -> &C {
        debug_assert!(index < self.len());
        // SAFETY: Bounds check deferred to the caller.
        unsafe { self.components.get_unchecked(index).as_ref() }
    }

    /// # Safety
    /// - The generic type parameter must match the underlying type of this component storage.
    pub unsafe fn get_mut<C: Component>(&mut self, index: usize) -> Option<&mut C> {
        // SAFETY: Bounds check deferred to the caller.
        unsafe { self.components.get(index).map(|ptr| ptr.as_mut()) }
    }

    /// # Safety
    /// - The index must be within the bounds of the underlying vec.
    /// - The generic type parameter must match the underlying type of this component storage.
    pub unsafe fn get_mut_unchecked<C: Component>(&mut self, index: usize) -> &mut C {
        debug_assert!(index < self.len());
        // SAFETY: Bounds check deferred to the caller.
        unsafe { self.components.get_unchecked(index).as_mut() }
    }

    /// # Panics
    /// Panics if the index is out of bounds.
    pub fn delete(&mut self, index: usize) {
        assert!(
            index < self.len(),
            "component index is {} but len is {}",
            index,
            self.len()
        );

        unsafe { self.delete_unchecked(index) }
    }

    /// # Safety
    /// - The index must be within the bounds of the underlying vec.
    pub unsafe fn delete_unchecked(&mut self, index: usize) {
        debug_assert!(index < self.len());

        // SAFETY: - We are correctly dropping the component
        //         - Deferred bounds check to the caller
        unsafe {
            // TODO: Consider implementing an internal `swap_drop` or something for ErasedVec.
            //       For now, `as_ptr().into()` coerces the associated lifetime of `ptr`, so that
            //       we can reborrow the ErasedVec as mutable to use the `dispose` method
            let ptr = self.components.swap_remove_unchecked(index).as_ptr().into();
            self.components.erased_type().dispose(ptr);
        }

        if self.is_tracked() {
            let tracker = self.get_tracker_mut_unchecked();
            tracker.delete(index);
        }
    }

    /// # Panics
    /// Panics if the `src_index` is out of bounds.
    pub fn move_component(&mut self, src_index: usize, dst: &mut Self) {
        assert!(
            src_index < self.len(),
            "component index is {} but len is {}",
            src_index,
            self.len()
        );

        unsafe { self.move_unchecked(src_index, dst) }
    }

    /// # Safety
    /// - The `src_index` must be within the bounds of the underlying source vec.
    pub unsafe fn move_unchecked(&mut self, src_index: usize, dst: &mut Self) {
        debug_assert!(src_index < self.len());

        // SAFETY: Bounds check deferred to the caller.
        unsafe {
            let ptr = self.components.swap_remove_unchecked(src_index);
            dst.components.push(ptr);
        }

        if self.is_tracked() {
            let tracker = self.get_tracker_mut_unchecked();
            tracker.delete(src_index);
        }

        if dst.is_tracked() {
            let tracker = dst.get_tracker_mut_unchecked();

            // TODO: we need to get current world tick to update last_write below
            let tick = 0;

            tracker.push(TrackingInfo::new(tick));
            tracker.last_write = tick;
        }
    }

    /// # Safety
    /// - The generic type parameter must match the underlying type of this component storage.
    pub unsafe fn iter<C: Component>(&self) -> core::slice::Iter<'_, UnsafeCell<C>> {
        unsafe { self.components.as_slice::<C>().iter() }
    }
}
