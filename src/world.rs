use crate::{
    archetype::ArchetypeManager,
    component::{Component, ComponentManager},
    entity::{Entity, EntityManager},
    event::{EventManager, Events},
    query::{bundle::ComponentBundle, QueryBuilder},
    resource::{Resource, ResourceId, ResourceManager},
    system::{schedule::Schedule, SystemManager},
};

pub struct World {
    pub(crate) entity_manager: EntityManager,
    pub(crate) archetype_manager: ArchetypeManager,
    pub(crate) component_manager: ComponentManager,
    pub(crate) system_manager: SystemManager,
    pub(crate) resource_manager: ResourceManager,
    pub(crate) event_manager: EventManager,
    pub tick: u32,
}

impl World {
    pub fn new() -> Self {
        Self {
            entity_manager: EntityManager::new(),
            archetype_manager: ArchetypeManager::new(),
            component_manager: ComponentManager::new(),
            system_manager: SystemManager::new(),
            resource_manager: ResourceManager::new(),
            event_manager: EventManager::new(),
            tick: 0,
        }
    }

    #[inline]
    pub fn create_entity(&mut self) -> Entity {
        let entity = self.entity_manager.create();

        // SAFETY: The entity is guaranteed to be alive as it was just created
        unsafe {
            self.archetype_manager
                .get_root_mut()
                .push_entity(entity, &mut self.entity_manager)
        };

        return entity;
    }

    #[inline]
    pub fn delete_entity(&mut self, entity: Entity) {
        if !self.entity_manager.alive(entity) {
            return;
        }

        self.archetype_manager
            .delete_entity(entity, &mut self.entity_manager);

        self.entity_manager.delete(entity)
    }

    #[inline]
    pub fn is_entity_alive(&self, entity: Entity) -> bool {
        self.entity_manager.alive(entity)
    }

    /// Registers the provided component in the current view, creating a corresponding component manager
    #[inline]
    pub fn register_component<C: Component>(&mut self) {
        self.component_manager.register::<C>()
    }

    #[inline]
    pub fn register_event<E: 'static>(&mut self) {
        let events = Events::<E>::new();
        let id = self.add_resource(events);
        self.event_manager.register_event(id);
    }

    pub fn has_component<C: Component>(&self, entity: Entity) -> bool {
        let Some(entity_record) = self.entity_manager.get_record(entity) else {
            return false;
        };

        let comp_id = self.component_manager.get_id::<C>();
        entity_record.archetype_id.test(comp_id)
    }

    /// Sets the provided component for the specified entity in the current view
    pub fn add_component<C: Component>(&mut self, entity: Entity, component: C) {
        if self.has_component::<C>(entity) {
            return;
        }

        self.archetype_manager.add_component(
            component,
            entity,
            &self.component_manager,
            &mut self.entity_manager,
        );
    }

    /// Removes the component of the specified type, for specified entity, in the current view
    pub fn remove_component<C: Component>(&mut self, entity: Entity) {
        if !self.has_component::<C>(entity) {
            return;
        }

        self.archetype_manager.remove_component::<C>(
            entity,
            &self.component_manager,
            &mut self.entity_manager,
        );
    }

    pub fn get_component<C: Component>(&self, entity: Entity) -> Option<&C> {
        let comp_id = self.component_manager.get_id::<C>();

        // This confirms that entity is alive
        let entity_record = self.entity_manager.get_record(entity)?;

        // SAFETY:
        // - If entity is alive, then archetype is guaranteed to be valid as it wrote its ID to the
        //   entity record in the first place.
        let arche = unsafe {
            self.archetype_manager
                .get_unchecked(&entity_record.archetype_id)
        };
        if !arche.has_component(comp_id) {
            return None;
        }

        // SAFETY:
        // - Archetype definitely contains component
        // - Component will be shared as reference so will not be dropped
        // - Entity is guaranteed to be alive, so row is valid as it will still be maintained by the archetype
        let component = unsafe { arche.get_component(comp_id, entity_record.archetype_row) };

        return Some(component);
    }

    pub fn get_component_mut<C: Component>(&mut self, entity: Entity) -> Option<&mut C> {
        let comp_id = self.component_manager.get_id::<C>();

        // This confirms that entity is alive
        let entity_record = self.entity_manager.get_record(entity)?;

        // SAFETY:
        // - If entity is alive, then archetype is guaranteed to be valid as it wrote its ID to the
        //   entity record in the first place.
        let arche = unsafe {
            self.archetype_manager
                .get_mut_unchecked(&entity_record.archetype_id)
        };
        if !arche.has_component(comp_id) {
            return None;
        }

        // SAFETY:
        // - Archetype definitely contains component
        // - Component will be shared as reference so will not be dropped
        // - Entity is guaranteed to be alive, so row is valid as it will still be maintained by the archetype
        let component = unsafe { arche.get_mut_component(comp_id, entity_record.archetype_row) };

        return Some(component);
    }

    pub fn add_resource<R: Resource>(&mut self, resource: R) -> ResourceId<R> {
        self.resource_manager.add(resource)
    }

    /// Although each Resource is guaranteed to be unique, the generic type parameter is only
    /// used to downcast the resource to the correct type. Instead the resource ID is used to
    /// locate the Resource for faster lookup.
    pub fn get_resource<R: Resource>(&self, id: ResourceId<R>) -> Option<&R> {
        self.resource_manager.get::<R>(id)
    }

    /// Although each Resource is guaranteed to be unique, the generic type parameter is only
    /// used to downcast the resource to the correct type. Instead the resource ID is used to
    /// locate the Resource for faster lookup.
    ///
    /// Notice that this method does not borrow &mut self, yet returns a mutable reference
    /// to the resource. This allows for multiple resources to be borrowed mutably at the same time,
    /// via internal use of UnsafeCell.
    ///
    /// # Safety
    /// - Mutable reference is obtained via UnsafeCell, so the resource must not be borrowed mutably elsewhere.
    pub unsafe fn get_mut_resource<R: Resource>(&self, id: ResourceId<R>) -> Option<&mut R> {
        unsafe { self.resource_manager.get_mut::<R>(id) }
    }

    /// Although each Resource is guaranteed to be unique, the generic type parameter is only
    /// used to downcast the resource to the correct type. Instead the resource ID is used to
    /// locate the Resource for faster lookup.
    ///
    /// # Safety
    /// - The ID must be valid, as no bounds check will be performed.
    pub unsafe fn get_resource_unchecked<R: Resource>(&self, id: ResourceId<R>) -> &R {
        unsafe { self.resource_manager.get_unchecked::<R>(id) }
    }

    /// Although each Resource is guaranteed to be unique, the generic type parameter is only
    /// used to downcast the resource to the correct type. Instead the resource ID is used to
    /// locate the Resource for faster lookup.
    ///
    /// Notice that this method does not borrow &mut self, yet returns a mutable reference
    /// to the resource. This allows for multiple resources to be borrowed mutably at the same time,
    /// via internal use of UnsafeCell.
    ///
    /// # Safety
    /// - The ID must be valid, as no bounds check will be performed.
    /// - This resource must not be borrowed mutably anywhere else.
    pub unsafe fn get_mut_resource_unchecked<R: Resource>(&self, id: ResourceId<R>) -> &mut R {
        unsafe { self.resource_manager.get_mut_unchecked::<R>(id) }
    }

    pub fn add_schedule(&mut self, schedule: Schedule) {
        self.system_manager.add(schedule);
    }

    pub fn query<C: ComponentBundle>(&mut self) -> QueryBuilder<'_, (C,)> {
        QueryBuilder::<(C,)>::new(
            &self.component_manager,
            &self.resource_manager,
            &mut self.archetype_manager,
        )
    }

    pub fn update(&mut self) {
        // TODO: Make this more efficient rather than cloning the system manager
        let mut system_manager = core::mem::replace(&mut self.system_manager, SystemManager::new());
        system_manager.update(self);
        self.system_manager = system_manager;
        // self.event_manager.clear_events(&self.resource_manager);
        self.tick += 1;
    }
}
