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
    ids: HashMap<TypeId, usize, nohash_hasher::BuildNoHashHasher<u64>>,
}

impl ResourceManager {
    pub fn new() -> Self {
        Self {
            resources: Vec::with_capacity(32),
            ids: HashMap::with_capacity_and_hasher(32, nohash_hasher::BuildNoHashHasher::default()),
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
        #[cold]
        #[inline(never)]
        #[track_caller]
        fn assert_failed<R>() -> ! {
            panic!(
                "Resource type {:?} not registered",
                std::any::type_name::<R>()
            );
        }

        let Some(&id) = self.ids.get(&TypeId::of::<R>()) else {
            assert_failed::<R>();
        };

        ResourceId::new(id)
    }

    pub fn get_storage(&self) -> &[Box<UnsafeCell<dyn Resource>>] {
        &self.resources
    }

    pub fn get<R: Resource>(&self, id: ResourceId<R>) -> Option<&R> {
        // SAFETY: ResourceId is created when inserting the resource, so type is guaranteed to be correct.
        unsafe {
            self.resources
                .get(id.index)
                .map(|r| r.get().cast::<R>().as_ref().unwrap_unchecked())
        }
    }

    /// # Safety
    /// - Mutable reference is obtained via UnsafeCell, so the resource must not be borrowed mutably elsewhere.
    pub unsafe fn get_mut<R: Resource>(&self, id: ResourceId<R>) -> Option<&mut R> {
        // SAFETY: ResourceId is created when inserting the resource, so type is guaranteed to be correct.
        unsafe {
            self.resources
                .get(id.index)
                .map(|r| r.get().cast::<R>().as_mut().unwrap_unchecked())
        }
    }

    /// # Safety
    /// - The key must be valid, and within bounds of the underlying vec.
    pub unsafe fn get_unchecked<R: Resource>(&self, id: ResourceId<R>) -> &R {
        // SAFETY: ResourceId is created when inserting the resource, so type is guaranteed to be correct.
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
    /// - The key must be valid, and within bounds of the underlying vec.
    /// - Mutable reference is obtained via UnsafeCell, so the resource must not be borrowed mutably elsewhere.
    pub unsafe fn get_mut_unchecked<R: Resource>(&self, id: ResourceId<R>) -> &mut R {
        // SAFETY: ResourceId is created when inserting the resource, so type is guaranteed to be correct.
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
