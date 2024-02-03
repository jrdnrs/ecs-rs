use core::cell::UnsafeCell;

use crate::{
    archetype::Archetype,
    component::{storage::ComponentStorage, Component, ComponentID, ComponentManager},
    entity::Entity,
    resource::{Resource, ResourceId, ResourceManager},
};

use super::filter::{And, FilterBuilder, Not, Tracked};

/// A ComponentBundle is a collection of one or more components that are used to
/// query the ECS for entities that have all of the components in the bundle.
///
/// Currently, [Entity] can also be included in a ComponentBundle, but this
/// may to change in the future.
pub trait ComponentBundle: 'static {
    /// The concrete component type that this parameter represents, but with a lifetime
    type Item<'a>;
    /// The collection from which an Item can be fetched
    type Storage<'a>: Copy;
    /// Identifier for the component type
    type Id: Copy;

    /// The number of components in the bundle
    fn count() -> usize {
        1
    }

    /// Returns the component type identifier for the parameter
    fn parameter_ids(component_manager: &ComponentManager) -> Self::Id;

    /// Contributes the component type to the filter, for matching with archetypes
    fn build_filter(filter: FilterBuilder, id: &Self::Id) -> FilterBuilder;

    /// Retrieves the component storage for the archetype
    fn prepare_storage<'a>(archetype: &'a Archetype, id: &Self::Id) -> Self::Storage<'a>;

    /// # Safety
    /// - The component type associated with the parameter must match the type of the Component Storage
    /// - The index must be within the bounds of the Component Storage
    unsafe fn fetch_item<'a>(storage: Self::Storage<'a>, index: usize) -> Self::Item<'a>;
}

impl ComponentBundle for () {
    type Item<'a> = ();
    type Storage<'a> = ();
    type Id = ();

    fn parameter_ids(component_manager: &ComponentManager) -> Self::Id {
        ()
    }

    fn build_filter(filter: FilterBuilder, id: &Self::Id) -> FilterBuilder {
        filter
    }

    fn prepare_storage<'a>(archetype: &'a Archetype, id: &Self::Id) -> Self::Storage<'a> {
        ()
    }

    unsafe fn fetch_item<'a>(storage: Self::Storage<'a>, index: usize) -> Self::Item<'a> {
        ()
    }
}

impl<T: Component> ComponentBundle for &'static T {
    type Item<'a> = &'a T;
    type Storage<'a> = &'a ComponentStorage;
    type Id = ComponentID;

    fn parameter_ids(component_manager: &ComponentManager) -> Self::Id {
        component_manager.get_id::<T>()
    }

    fn build_filter(filter: FilterBuilder, id: &Self::Id) -> FilterBuilder {
        filter.and(*id)
    }

    fn prepare_storage<'a>(archetype: &'a Archetype, id: &Self::Id) -> Self::Storage<'a> {
        unsafe { archetype.get_storage(*id) }
    }

    unsafe fn fetch_item<'a>(storage: Self::Storage<'a>, index: usize) -> Self::Item<'a> {
        storage.get_as_ptr(index).as_ref::<T>()
    }
}

impl<T: Component> ComponentBundle for &'static mut T {
    type Item<'a> = &'a mut T;
    type Storage<'a> = &'a ComponentStorage;
    type Id = ComponentID;

    fn parameter_ids(component_manager: &ComponentManager) -> Self::Id {
        component_manager.get_id::<T>()
    }

    fn build_filter(filter: FilterBuilder, id: &Self::Id) -> FilterBuilder {
        filter.and(*id)
    }

    fn prepare_storage<'a>(archetype: &'a Archetype, id: &Self::Id) -> Self::Storage<'a> {
        unsafe { archetype.get_storage(*id) }
    }

    unsafe fn fetch_item<'a>(storage: Self::Storage<'a>, index: usize) -> Self::Item<'a> {
        storage.get_as_ptr(index).as_mut::<T>()
    }
}

impl<T: Component> ComponentBundle for Option<&'static T> {
    type Item<'a> = Option<&'a T>;
    type Storage<'a> = Option<&'a ComponentStorage>;
    type Id = ComponentID;

    fn parameter_ids(component_manager: &ComponentManager) -> Self::Id {
        component_manager.get_id::<T>()
    }

    fn build_filter(filter: FilterBuilder, id: &Self::Id) -> FilterBuilder {
        // This parameter is optional, so we want the archetype to match even if the component is not present
        filter
    }

    fn prepare_storage<'a>(archetype: &'a Archetype, id: &Self::Id) -> Self::Storage<'a> {
        if archetype.has_component(*id) {
            unsafe { Some(archetype.get_storage(*id)) }
        } else {
            None
        }
    }

    unsafe fn fetch_item<'a>(storage: Self::Storage<'a>, index: usize) -> Self::Item<'a> {
        storage.map(|storage| storage.get_as_ptr(index).as_ref::<T>())
    }
}

impl<T: Component> ComponentBundle for Option<&'static mut T> {
    type Item<'a> = Option<&'a mut T>;
    type Storage<'a> = Option<&'a ComponentStorage>;
    type Id = ComponentID;

    fn parameter_ids(component_manager: &ComponentManager) -> Self::Id {
        component_manager.get_id::<T>()
    }

    fn build_filter(filter: FilterBuilder, id: &Self::Id) -> FilterBuilder {
        // This parameter is optional, so we want the archetype to match even if the component is not present
        filter
    }

    fn prepare_storage<'a>(archetype: &'a Archetype, id: &Self::Id) -> Self::Storage<'a> {
        if archetype.has_component(*id) {
            unsafe { Some(archetype.get_storage(*id)) }
        } else {
            None
        }
    }

    unsafe fn fetch_item<'a>(storage: Self::Storage<'a>, index: usize) -> Self::Item<'a> {
        storage.map(|storage| storage.get_as_ptr(index).as_mut::<T>())
    }
}

impl<T: Component> ComponentBundle for Tracked<&'static T> {
    type Item<'a> = Tracked<&'a T>;
    type Storage<'a> = &'a ComponentStorage;
    type Id = ComponentID;

    fn parameter_ids(component_manager: &ComponentManager) -> Self::Id {
        component_manager.get_id::<T>()
    }

    fn build_filter(filter: FilterBuilder, id: &Self::Id) -> FilterBuilder {
        let filter = filter.and(*id);
        let filter = filter.track(*id);
        filter
    }

    fn prepare_storage<'a>(archetype: &'a Archetype, id: &Self::Id) -> Self::Storage<'a> {
        unsafe { archetype.get_storage(*id) }
    }

    unsafe fn fetch_item<'a>(storage: Self::Storage<'a>, index: usize) -> Self::Item<'a> {
        let tracker = storage.tracker.as_ref().unwrap_unchecked();
        let item_info = tracker.get(index);
        let item = storage.get_as_ptr(index).as_ref::<T>();

        // If we are reading this component a single tick after it was modified, the `modified` and `read` ticks
        // will be equal. This does not mean it was modified in this current tick - `read` is updated **after**
        // all systems have been executed, so it is the tick it was *last* read.
        if item_info.modified >= tracker.last_read {
            Tracked::Modified(item)
        } else {
            Tracked::Unmodified(item)
        }
    }
}

impl<T: Component> ComponentBundle for Tracked<&'static mut T> {
    type Item<'a> = Tracked<&'a mut T>;
    type Storage<'a> = &'a ComponentStorage;
    type Id = ComponentID;

    fn parameter_ids(component_manager: &ComponentManager) -> Self::Id {
        component_manager.get_id::<T>()
    }

    fn build_filter(filter: FilterBuilder, id: &Self::Id) -> FilterBuilder {
        let filter = filter.and(*id);
        let filter = filter.track(*id);
        filter
    }

    fn prepare_storage<'a>(archetype: &'a Archetype, id: &Self::Id) -> Self::Storage<'a> {
        unsafe { archetype.get_storage(*id) }
    }

    unsafe fn fetch_item<'a>(storage: Self::Storage<'a>, index: usize) -> Self::Item<'a> {
        let tracker = storage.tracker.as_ref().unwrap_unchecked();
        let item_info = tracker.get(index);
        let item = storage.get_as_ptr(index).as_mut::<T>();

        // If we are reading this component a single tick after it was modified, the `modified` and `read` ticks
        // will be equal. This does not mean it was modified in this current tick - `read` is updated **after**
        // all systems have been executed, so it is the tick it was *last* read.
        if item_info.modified >= tracker.last_read {
            Tracked::Modified(item)
        } else {
            Tracked::Unmodified(item)
        }
    }
}

impl<T: Component> ComponentBundle for Not<T> {
    type Item<'a> = T;
    type Storage<'a> = &'a ComponentStorage;
    type Id = ComponentID;

    fn parameter_ids(component_manager: &ComponentManager) -> Self::Id {
        component_manager.get_id::<T>()
    }

    fn build_filter(filter: FilterBuilder, id: &Self::Id) -> FilterBuilder {
        filter.not(*id)
    }

    fn prepare_storage<'a>(archetype: &'a Archetype, id: &Self::Id) -> Self::Storage<'a> {
        unimplemented!()
    }

    unsafe fn fetch_item<'a>(storage: Self::Storage<'a>, index: usize) -> Self::Item<'a> {
        unimplemented!()
    }
}

impl<T: Component> ComponentBundle for And<T> {
    type Item<'a> = T;
    type Storage<'a> = &'a ComponentStorage;
    type Id = ComponentID;

    fn parameter_ids(component_manager: &ComponentManager) -> Self::Id {
        component_manager.get_id::<T>()
    }

    fn build_filter(filter: FilterBuilder, id: &Self::Id) -> FilterBuilder {
        filter.and(*id)
    }

    fn prepare_storage<'a>(archetype: &'a Archetype, id: &Self::Id) -> Self::Storage<'a> {
        unimplemented!()
    }

    unsafe fn fetch_item<'a>(storage: Self::Storage<'a>, index: usize) -> Self::Item<'a> {
        unimplemented!()
    }
}

impl ComponentBundle for Entity {
    type Item<'a> = Entity;
    type Storage<'a> = &'a Vec<Entity>;
    type Id = usize;

    fn count() -> usize {
        0
    }

    fn parameter_ids(_component_manager: &ComponentManager) -> Self::Id {
        // Entity does not have an id
        usize::MAX
    }

    fn build_filter(filter: FilterBuilder, id: &Self::Id) -> FilterBuilder {
        // Entity is always present, so we don't need to add it to the filter
        filter
    }

    fn prepare_storage<'a>(archetype: &'a Archetype, _id: &Self::Id) -> Self::Storage<'a> {
        &archetype.entities
    }

    unsafe fn fetch_item<'a>(storage: Self::Storage<'a>, index: usize) -> Self::Item<'a> {
        *storage.get_unchecked(index)
    }
}

impl<P1: ComponentBundle, P2: ComponentBundle> ComponentBundle for (P1, P2) {
    type Item<'a> = (P1::Item<'a>, P2::Item<'a>);
    type Storage<'a> = (P1::Storage<'a>, P2::Storage<'a>);
    type Id = (P1::Id, P2::Id);

    fn count() -> usize {
        P1::count() + P2::count()
    }

    fn parameter_ids(component_manager: &ComponentManager) -> Self::Id {
        (
            P1::parameter_ids(component_manager),
            P2::parameter_ids(component_manager),
        )
    }

    fn build_filter(filter: FilterBuilder, id: &Self::Id) -> FilterBuilder {
        let filter = P1::build_filter(filter, &id.0);
        let filter = P2::build_filter(filter, &id.1);
        filter
    }

    fn prepare_storage<'a>(archetype: &'a Archetype, id: &Self::Id) -> Self::Storage<'a> {
        (
            P1::prepare_storage(archetype, &id.0),
            P2::prepare_storage(archetype, &id.1),
        )
    }

    unsafe fn fetch_item<'a>(storage: Self::Storage<'a>, index: usize) -> Self::Item<'a> {
        (
            P1::fetch_item(storage.0, index),
            P2::fetch_item(storage.1, index),
        )
    }
}

impl<P1: ComponentBundle, P2: ComponentBundle, P3: ComponentBundle> ComponentBundle
    for (P1, P2, P3)
{
    type Item<'a> = (P1::Item<'a>, P2::Item<'a>, P3::Item<'a>);
    type Storage<'a> = (P1::Storage<'a>, P2::Storage<'a>, P3::Storage<'a>);
    type Id = (P1::Id, P2::Id, P3::Id);

    fn count() -> usize {
        P1::count() + P2::count() + P3::count()
    }

    fn parameter_ids(component_manager: &ComponentManager) -> Self::Id {
        (
            P1::parameter_ids(component_manager),
            P2::parameter_ids(component_manager),
            P3::parameter_ids(component_manager),
        )
    }

    fn build_filter(filter: FilterBuilder, id: &Self::Id) -> FilterBuilder {
        let filter = P1::build_filter(filter, &id.0);
        let filter = P2::build_filter(filter, &id.1);
        let filter = P3::build_filter(filter, &id.2);
        filter
    }

    fn prepare_storage<'a>(archetype: &'a Archetype, id: &Self::Id) -> Self::Storage<'a> {
        (
            P1::prepare_storage(archetype, &id.0),
            P2::prepare_storage(archetype, &id.1),
            P3::prepare_storage(archetype, &id.2),
        )
    }

    unsafe fn fetch_item<'a>(storage: Self::Storage<'a>, index: usize) -> Self::Item<'a> {
        (
            P1::fetch_item(storage.0, index),
            P2::fetch_item(storage.1, index),
            P3::fetch_item(storage.2, index),
        )
    }
}

/// A ResourceBundle is a collection of resources that can be fetched from a resource manager.
pub trait ResourceBundle: 'static {
    /// The concrete type of the Resource, but with a lifetime
    type Item<'a>;
    /// Identifier of the resource
    type Id: Copy;

    /// Returns the identifier of the resource
    fn parameter_ids(resource_manager: &ResourceManager) -> Self::Id;

    /// # Safety
    /// - The generic type must be the same as the one used to register the resource
    /// - The index must be within the bounds
    unsafe fn fetch_item<'a>(
        storage: &'a [Box<UnsafeCell<dyn Resource>>],
        key: Self::Id,
    ) -> Self::Item<'a>;
}

impl ResourceBundle for () {
    type Item<'a> = ();
    type Id = ();

    fn parameter_ids(_resource_manager: &ResourceManager) -> Self::Id {
        ()
    }

    unsafe fn fetch_item<'a>(
        _storage: &'a [Box<UnsafeCell<dyn Resource>>],
        _key: Self::Id,
    ) -> Self::Item<'a> {
        ()
    }
}

impl<R: Resource> ResourceBundle for &'static R {
    type Item<'a> = &'a R;
    type Id = ResourceId<R>;

    fn parameter_ids(resource_manager: &ResourceManager) -> Self::Id {
        resource_manager.get_id::<R>()
    }

    unsafe fn fetch_item<'a>(
        storage: &'a [Box<UnsafeCell<dyn Resource>>],
        key: Self::Id,
    ) -> Self::Item<'a> {
        // SAFETY: The caller must ensure the type is correct.
        unsafe {
            storage
                .get_unchecked(key.index)
                .get()
                .cast::<R>()
                .as_ref()
                .unwrap_unchecked()
        }
    }
}

impl<R: Resource> ResourceBundle for &'static mut R {
    type Item<'a> = &'a mut R;
    type Id = ResourceId<R>;

    fn parameter_ids(resource_manager: &ResourceManager) -> Self::Id {
        resource_manager.get_id::<R>()
    }

    unsafe fn fetch_item<'a>(
        storage: &'a [Box<UnsafeCell<dyn Resource>>],
        key: ResourceId<R>,
    ) -> Self::Item<'a> {
        // SAFETY: The caller must ensure the type is correct.
        unsafe {
            storage
                .get_unchecked(key.index)
                .get()
                .cast::<R>()
                .as_mut()
                .unwrap_unchecked()
        }
    }
}

impl<R1: ResourceBundle, R2: ResourceBundle> ResourceBundle for (R1, R2) {
    type Item<'a> = (R1::Item<'a>, R2::Item<'a>);
    type Id = (R1::Id, R2::Id);

    fn parameter_ids(resource_manager: &ResourceManager) -> Self::Id {
        (
            R1::parameter_ids(resource_manager),
            R2::parameter_ids(resource_manager),
        )
    }

    unsafe fn fetch_item<'a>(
        storage: &'a [Box<UnsafeCell<dyn Resource>>],
        key: Self::Id,
    ) -> Self::Item<'a> {
        (
            R1::fetch_item(storage, key.0),
            R2::fetch_item(storage, key.1),
        )
    }
}

impl<R1: ResourceBundle, R2: ResourceBundle, R3: ResourceBundle> ResourceBundle for (R1, R2, R3) {
    type Item<'a> = (R1::Item<'a>, R2::Item<'a>, R3::Item<'a>);
    type Id = (R1::Id, R2::Id, R3::Id);

    fn parameter_ids(resource_manager: &ResourceManager) -> Self::Id {
        (
            R1::parameter_ids(resource_manager),
            R2::parameter_ids(resource_manager),
            R3::parameter_ids(resource_manager),
        )
    }

    unsafe fn fetch_item<'a>(
        storage: &'a [Box<UnsafeCell<dyn Resource>>],
        key: Self::Id,
    ) -> Self::Item<'a> {
        (
            R1::fetch_item(storage, key.0),
            R2::fetch_item(storage, key.1),
            R3::fetch_item(storage, key.2),
        )
    }
}

impl<R1: ResourceBundle, R2: ResourceBundle, R3: ResourceBundle, R4: ResourceBundle> ResourceBundle
    for (R1, R2, R3, R4)
{
    type Item<'a> = (R1::Item<'a>, R2::Item<'a>, R3::Item<'a>, R4::Item<'a>);
    type Id = (R1::Id, R2::Id, R3::Id, R4::Id);

    fn parameter_ids(resource_manager: &ResourceManager) -> Self::Id {
        (
            R1::parameter_ids(resource_manager),
            R2::parameter_ids(resource_manager),
            R3::parameter_ids(resource_manager),
            R4::parameter_ids(resource_manager),
        )
    }

    unsafe fn fetch_item<'a>(
        storage: &'a [Box<UnsafeCell<dyn Resource>>],
        key: Self::Id,
    ) -> Self::Item<'a> {
        (
            R1::fetch_item(storage, key.0),
            R2::fetch_item(storage, key.1),
            R3::fetch_item(storage, key.2),
            R4::fetch_item(storage, key.3),
        )
    }
}

impl<
        R1: ResourceBundle,
        R2: ResourceBundle,
        R3: ResourceBundle,
        R4: ResourceBundle,
        R5: ResourceBundle,
    > ResourceBundle for (R1, R2, R3, R4, R5)
{
    type Item<'a> = (
        R1::Item<'a>,
        R2::Item<'a>,
        R3::Item<'a>,
        R4::Item<'a>,
        R5::Item<'a>,
    );
    type Id = (R1::Id, R2::Id, R3::Id, R4::Id, R5::Id);

    fn parameter_ids(resource_manager: &ResourceManager) -> Self::Id {
        (
            R1::parameter_ids(resource_manager),
            R2::parameter_ids(resource_manager),
            R3::parameter_ids(resource_manager),
            R4::parameter_ids(resource_manager),
            R5::parameter_ids(resource_manager),
        )
    }

    unsafe fn fetch_item<'a>(
        storage: &'a [Box<UnsafeCell<dyn Resource>>],
        key: Self::Id,
    ) -> Self::Item<'a> {
        (
            R1::fetch_item(storage, key.0),
            R2::fetch_item(storage, key.1),
            R3::fetch_item(storage, key.2),
            R4::fetch_item(storage, key.3),
            R5::fetch_item(storage, key.4),
        )
    }
}

impl<
        R1: ResourceBundle,
        R2: ResourceBundle,
        R3: ResourceBundle,
        R4: ResourceBundle,
        R5: ResourceBundle,
        R6: ResourceBundle,
    > ResourceBundle for (R1, R2, R3, R4, R5, R6)
{
    type Item<'a> = (
        R1::Item<'a>,
        R2::Item<'a>,
        R3::Item<'a>,
        R4::Item<'a>,
        R5::Item<'a>,
        R6::Item<'a>,
    );
    type Id = (R1::Id, R2::Id, R3::Id, R4::Id, R5::Id, R6::Id);

    fn parameter_ids(resource_manager: &ResourceManager) -> Self::Id {
        (
            R1::parameter_ids(resource_manager),
            R2::parameter_ids(resource_manager),
            R3::parameter_ids(resource_manager),
            R4::parameter_ids(resource_manager),
            R5::parameter_ids(resource_manager),
            R6::parameter_ids(resource_manager),
        )
    }

    unsafe fn fetch_item<'a>(
        storage: &'a [Box<UnsafeCell<dyn Resource>>],
        key: Self::Id,
    ) -> Self::Item<'a> {
        (
            R1::fetch_item(storage, key.0),
            R2::fetch_item(storage, key.1),
            R3::fetch_item(storage, key.2),
            R4::fetch_item(storage, key.3),
            R5::fetch_item(storage, key.4),
            R6::fetch_item(storage, key.5),
        )
    }
}
