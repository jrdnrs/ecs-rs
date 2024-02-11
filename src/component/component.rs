use core::{
    alloc::Layout,
    any::{Any, TypeId},
};
use std::collections::HashMap;

use collections::Ptr;

/// Unique sequential integer
pub type ComponentID = usize;

/// Stores all component data, organised by component type into component storages
pub struct ComponentManager {
    /// Used to translate component type ids to component ids
    ids: HashMap<TypeId, ComponentID, nohash_hasher::BuildNoHashHasher<u64>>,

    /// Stores the metadata for each component type, accessible using the component id
    /// as the index
    metadata: Vec<ComponentMetaData>,
}

impl ComponentManager {
    pub fn new() -> Self {
        Self {
            ids: HashMap::with_capacity_and_hasher(8, nohash_hasher::BuildNoHashHasher::default()),
            metadata: Vec::with_capacity(8),
        }
    }

    /// Registers a component type with the component manager
    pub fn register<C: Component>(&mut self) {
        let type_id = C::type_id();
        if self.ids.contains_key(&type_id) {
            return;
        }

        let comp_id = self.ids.len();
        self.ids.insert(type_id, comp_id);
        self.metadata.push(ComponentMetaData::new::<C>());
    }

    /// Returns the component id for the given component type
    /// # Panics
    /// - If the component type is not registered
    pub fn get_id<C: Component>(&self) -> ComponentID {
        #[cold]
        #[inline(never)]
        #[track_caller]
        fn assert_failed<C>() -> ! {
            panic!(
                "Component type {:?} not registered",
                std::any::type_name::<C>()
            );
        }

        let Some(&id) = self.ids.get(&TypeId::of::<C>()) else {
            assert_failed::<C>();
        };

        id
    }

    /// Returns the component layout for the given component type
    pub fn get_metadata(&self, comp_id: ComponentID) -> &ComponentMetaData {
        &self.metadata[comp_id]
    }
}

pub struct ComponentMetaData {
    pub type_id: TypeId,
    pub layout: Layout,
    pub drop: unsafe fn(Ptr),
}

impl ComponentMetaData {
    pub fn new<T: Component>() -> Self {
        Self {
            type_id: T::type_id(),
            layout: Layout::new::<T>(),
            drop: |ptr: Ptr| unsafe { ptr.drop_as::<T>() },
        }
    }
}

pub trait Component: 'static {
    /// Returns the type id of the component type
    fn type_id() -> TypeId {
        TypeId::of::<Self>()
    }
}
impl<T: Any> Component for T {}

#[cfg(test)]
mod tests {
    use crate::component::storage::ComponentStorage;

    use super::*;

    type CompA = u32;
    type CompB = u64;

    #[test]
    fn component_registration() {
        let mut manager = ComponentManager::new();
        manager.register::<CompA>();
        manager.register::<CompB>();

        assert_eq!(manager.get_id::<CompA>(), 0);
        assert_eq!(manager.get_id::<CompB>(), 1);
    }

    #[test]
    fn push_component() {
        let mut manager = ComponentManager::new();
        manager.register::<CompA>();
        manager.register::<CompB>();

        let mut storage = ComponentStorage::new::<CompA>(0);
        unsafe { storage.push(42) };

        assert_eq!(unsafe { storage.get::<CompA>(0) }, &42);
    }

    #[test]
    fn delete_component() {
        let mut manager = ComponentManager::new();
        manager.register::<CompA>();
        manager.register::<CompB>();

        let mut storage = ComponentStorage::new::<CompA>(0);
        unsafe { storage.push(42) };
        unsafe { storage.delete(0) };

        assert_eq!(storage.components.len(), 0);
    }

    #[test]
    fn move_component() {
        let mut manager = ComponentManager::new();
        manager.register::<CompA>();
        manager.register::<CompB>();

        let mut storage = ComponentStorage::new::<CompA>(0);
        unsafe { storage.push(42) };

        let mut other = ComponentStorage::new::<CompA>(1);
        unsafe { storage.transfer(0, &mut other) };

        assert_eq!(storage.components.len(), 0);
        assert_eq!(unsafe { other.get::<CompA>(0) }, &42);
    }
}
