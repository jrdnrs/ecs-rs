use core::{
    any::{Any, TypeId},
    cell::UnsafeCell,
};
use std::collections::HashMap;

pub trait Resource: 'static {}
impl<T: Any> Resource for T {}

pub struct ResourceId<R: Resource> {
    pub(crate) index: usize,
    _marker: std::marker::PhantomData<R>,
}

// Manual impl needed because of PhantomData
impl<R: Resource> Copy for ResourceId<R> {}
impl<R: Resource> Clone for ResourceId<R> {
    fn clone(&self) -> ResourceId<R> {
        *self
    }
}

impl<R: Resource> ResourceId<R> {
    pub(crate) fn new(id: usize) -> Self {
        Self {
            index: id,
            _marker: std::marker::PhantomData,
        }
    }
}

/// A collection of resources
///
/// # Implementation
/// Resources are stored in a vec of heap allocated trait objects, though these are expected
/// to be downcasted to their concrete types immediately after being retrieved. This is possible
/// with the use of [ResourceId]s, which are used as indices into the vec, as they are associated
/// with the type of the resource they point to.
///
/// Resources can be fetched by their type, or by their [ResourceId]. The latter is more efficient,
/// as the former requires use of a hashmap to find the index of the resource in the vec, using the
/// [TypeId].
///
/// Currently, [UnsafeCell] is used as a workaround to easily get mutable references to several
/// resources at once.
pub struct ResourceManager {
    resources: Vec<Box<UnsafeCell<dyn Resource>>>,
    ids: HashMap<TypeId, usize>,
}

impl ResourceManager {
    pub fn new() -> Self {
        Self {
            resources: Vec::new(),
            ids: HashMap::new(),
        }
    }

    pub fn add<R: Resource>(&mut self, resource: R) -> ResourceId<R> {
        let index = self.resources.len();
        let type_id = TypeId::of::<R>();
        self.ids.insert(type_id, index);

        self.resources.push(Box::new(UnsafeCell::new(resource)));

        return ResourceId::new(index);
    }

    pub fn get_id<R: Resource>(&self) -> ResourceId<R> {
        ResourceId::new(*self.ids.get(&TypeId::of::<R>()).unwrap())
    }

    pub fn get_storage(&self) -> &[Box<UnsafeCell<dyn Resource>>] {
        &self.resources
    }

    /// # Safety
    /// - The generic type parameter must match the underlying type of this resource.
    pub unsafe fn get<R: Resource>(&self, id: ResourceId<R>) -> Option<&R> {
        // SAFETY: The caller must ensure the type is correct.
        unsafe {
            self.resources
                .get(id.index)
                .map(|r| r.get().cast::<R>().as_ref().unwrap_unchecked())
        }
    }

    /// # Safety
    /// - The generic type parameter must match the underlying type of this resource.
    pub unsafe fn get_mut<R: Resource>(&self, id: ResourceId<R>) -> Option<&mut R> {
        // SAFETY: The caller must ensure the type is correct.
        unsafe {
            self.resources
                .get(id.index)
                .map(|r| r.get().cast::<R>().as_mut().unwrap_unchecked())
        }
    }

    /// # Safety
    /// - The generic type parameter must match the underlying type of this resource.
    /// - The key must be valid, and within bounds of the underlying vec.
    pub unsafe fn get_unchecked<R: Resource>(&self, id: ResourceId<R>) -> &R {
        // SAFETY: Type and bounds checks are deferred to the caller.
        unsafe {
            self.resources
                .get_unchecked(id.index)
                .get()
                .cast::<R>()
                .as_ref()
                .unwrap_unchecked()
        }
    }

    /// # Safety
    /// - The generic type parameter must match the underlying type of this resource.
    /// - The key must be valid, and within bounds of the underlying vec.
    pub unsafe fn get_mut_unchecked<R: Resource>(&self, id: ResourceId<R>) -> &mut R {
        // SAFETY: Type and bounds checks are deferred to the caller.
        unsafe {
            self.resources
                .get_unchecked(id.index)
                .get()
                .cast::<R>()
                .as_mut()
                .unwrap_unchecked()
        }
    }
}
