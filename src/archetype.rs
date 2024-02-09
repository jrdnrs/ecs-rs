use std::collections::HashMap;

use collections::{BitSet, SparseMap};

use crate::{
    component::{storage::ComponentStorage, Component, ComponentID, ComponentManager},
    entity::{Entity, EntityManager},
};

pub type ArchetypeID = BitSet;

pub struct Archetype {
    /// The ID of an archetype is a bitset that represents the component IDs that are present within,
    /// where the index of each set bit corresponds to the component ID.
    pub id: ArchetypeID,

    /// Values in this map are IDs for other Archetypes that match the current archetype, but with
    /// the addition or removal of a single component. The key is the component ID that is added or
    /// removed to get to the other archetype.
    pub edges: SparseMap<ArchetypeID>,

    /// Values in this map are the component storage for each component that is present within the
    /// archetype. The key is the component ID.
    pub components: SparseMap<ComponentStorage>,

    /// The entities that are present within the archetype. The index of each entity in this vec
    /// corresponds to the row of the entity within the component storages.
    pub entities: Vec<Entity>,
}

impl Archetype {
    pub fn new(id: BitSet) -> Self {
        Self {
            id,
            edges: SparseMap::with_capacity(4),
            components: SparseMap::with_capacity(4),
            entities: Vec::with_capacity(8),
        }
    }

    pub fn comp_ids(&self) -> impl Iterator<Item = &ComponentID> {
        self.components.keys()
    }

    pub fn has_component(&self, comp_id: ComponentID) -> bool {
        self.id.test(comp_id)
    }

    /// # Safety
    /// - The component IDs must exist within this archetype as well as the destination archetype, as no bounds
    ///   checking is performed.
    /// - The entity must be alive, and only exists in this archetype.
    pub unsafe fn move_entity<'a>(
        &mut self,
        entity: Entity,
        comp_ids: impl Iterator<Item = &'a ComponentID>,
        dst_arche: &mut Archetype,
        entity_manager: &mut EntityManager,
    ) {
        // SAFETY: As long as the above invariants are upheld, this is all safe
        let entity_record = unsafe { entity_manager.get_record_unchecked(entity) };

        for comp_id in comp_ids {
            unsafe { self.move_component(*comp_id, entity_record.archetype_row, dst_arche) };
        }

        unsafe { self.delete_entity(entity, entity_manager) };
        unsafe { dst_arche.push_entity(entity, entity_manager) };
    }

    /// # Safety
    /// - The entity must be alive, and does not already exist in this archetype.
    pub unsafe fn push_entity(&mut self, entity: Entity, entity_manager: &mut EntityManager) {
        // SAFETY: As long as the above invariants are upheld, this is all safe
        let entity_record = unsafe { entity_manager.get_record_mut_unchecked(entity) };

        entity_record.archetype_row = self.entities.len();
        entity_record.archetype_id = self.id.clone();
        self.entities.push(entity);
    }

    /// # Safety
    /// - The entity must be alive, and only exists in this archetype.
    pub unsafe fn delete_entity(&mut self, entity: Entity, entity_manager: &mut EntityManager) {
        // SAFETY: As long as the above invariants are upheld, this is all safe
        let entity_row = unsafe { entity_manager.get_record_unchecked(entity).archetype_row };

        // SAFETY: This entity (at end of vec) already exists in this archetype, so the row is assumed to be valid
        unsafe {
            entity_manager
                .get_record_mut_unchecked(self.entities[self.entities.len() - 1])
                .archetype_row = entity_row;
        }

        self.entities.swap_remove(entity_row);
    }

    /// # Safety
    /// - The concrete type associated with the component must match the type of an underlying
    ///   component storage within this archetype.
    /// - The component ID must exist within this archetype, as no bounds checking is performed.
    pub unsafe fn push_component<C: Component>(&mut self, comp_id: ComponentID, component: C) {
        // SAFETY: Deferred to the caller
        let storage = unsafe { self.get_mut_storage(comp_id) };
        unsafe { storage.push(component) };
    }

    /// # Safety
    /// - The component ID must exist within this archetype, as no bounds checking is performed.
    /// - The row must be within the bounds of the underlying vec.
    pub unsafe fn delete_component(&mut self, comp_id: ComponentID, row: usize) {
        // SAFETY: Deferred to the caller
        let storage = unsafe { self.get_mut_storage(comp_id) };
        unsafe { storage.delete_unchecked(row) }
    }

    /// # Safety
    /// - The component ID must exist within this archetype, as no bounds checking is performed.
    /// - The row must be within the bounds of the underlying vec.
    pub unsafe fn get_component<T: Component>(&self, comp_id: ComponentID, row: usize) -> &T {
        // SAFETY: Deferred to the caller
        let storage = unsafe { self.get_storage(comp_id) };
        unsafe { storage.get_unchecked(row) }
    }

    /// # Safety
    /// - The component ID must exist within this archetype, as no bounds checking is performed.
    /// - The row must be within the bounds of the underlying vec.
    pub unsafe fn get_mut_component<T: Component>(
        &mut self,
        comp_id: ComponentID,
        row: usize,
    ) -> &mut T {
        // SAFETY: Deferred to the caller
        let storage = unsafe { self.get_mut_storage(comp_id) };
        unsafe { storage.get_mut_unchecked(row) }
    }

    /// # Safety
    /// - The component ID must exist within this archetype as well as the destination archetype, as no bounds
    ///   checking is performed.
    /// - The `src_row` must be within the bounds of the underlying vec.
    pub unsafe fn move_component(
        &mut self,
        comp_id: ComponentID,
        src_row: usize,
        dst_arche: &mut Self,
    ) {
        // SAFETY: Deferred to the caller
        let src_storage = unsafe { self.get_mut_storage(comp_id) };
        let dst_storage = unsafe { dst_arche.get_mut_storage(comp_id) };

        unsafe { src_storage.move_unchecked(src_row, dst_storage) }
    }

    /// # Safety
    /// - The component ID must exist within this archetype, as no bounds checking is performed.
    pub unsafe fn get_storage(&self, comp_id: ComponentID) -> &ComponentStorage {
        debug_assert!(
            self.id.test(comp_id),
            "Component ID does not match archetype"
        );

        unsafe { self.components.get_unchecked(comp_id) }
    }

    /// # Safety
    /// - The component ID must exist within this archetype, as no bounds checking is performed.
    pub unsafe fn get_mut_storage(&mut self, comp_id: ComponentID) -> &mut ComponentStorage {
        debug_assert!(
            self.id.test(comp_id),
            "Component ID does not match archetype"
        );

        unsafe { self.components.get_mut_unchecked(comp_id) }
    }
}

pub struct ArchetypeManager {
    /// The root archetype is the archetype that all other archetypes are derived from, as it contains
    /// no components! All entities start their life in the root archetype, and are moved to other archetypes
    /// as they gain components.
    root_archetype: ArchetypeID,

    /// A table of all archetypes that exist within the world.
    pub archetype_table: HashMap<ArchetypeID, Archetype, ahash::RandomState>,

    /// When a query is first created, archetypes relevant to that query are cached. If a new archetype
    /// is created it is added to this queue so that, after all systems have run for a given world update,
    /// they can be checked for relevance to every query and added to their cache, before being cleared.
    pub new_archetypes_queue: Vec<ArchetypeID>,
}

impl ArchetypeManager {
    pub fn new() -> Self {
        let root_id = BitSet::new();
        let root = Archetype::new(root_id.clone());
        let mut archetype_table =
            HashMap::with_capacity_and_hasher(4, ahash::RandomState::default());
        archetype_table.insert(root_id.clone(), root);

        Self {
            root_archetype: root_id,
            archetype_table,
            new_archetypes_queue: Vec::new(),
        }
    }

    pub fn archetype_ids(&self) -> impl Iterator<Item = &ArchetypeID> {
        self.archetype_table.keys()
    }

    pub fn archetypes(&self) -> impl Iterator<Item = &Archetype> {
        self.archetype_table.values()
    }

    pub fn archetypes_mut(&mut self) -> impl Iterator<Item = &mut Archetype> {
        self.archetype_table.values_mut()
    }

    pub fn get_root(&self) -> &Archetype {
        // SAFETY: The root archetype is always present
        unsafe {
            self.archetype_table
                .get(&self.root_archetype)
                .unwrap_unchecked()
        }
    }

    pub fn get_root_mut(&mut self) -> &mut Archetype {
        // SAFETY: The root archetype is always present
        unsafe {
            self.archetype_table
                .get_mut(&self.root_archetype)
                .unwrap_unchecked()
        }
    }

    pub fn get(&self, arche_id: &ArchetypeID) -> Option<&Archetype> {
        self.archetype_table.get(arche_id)
    }

    /// # Safety
    /// - The archetype ID must exist within this manager, as no existence check is performed.
    pub unsafe fn get_unchecked(&self, arche_id: &ArchetypeID) -> &Archetype {
        debug_assert!(self.archetype_table.contains_key(arche_id));
        unsafe { self.archetype_table.get(arche_id).unwrap_unchecked() }
    }

    pub fn get_mut(&mut self, arche_id: &ArchetypeID) -> Option<&mut Archetype> {
        self.archetype_table.get_mut(arche_id)
    }

    /// # Safety
    /// - The archetype ID must exist within this manager, as no existence check is performed.
    pub unsafe fn get_mut_unchecked(&mut self, arche_id: &ArchetypeID) -> &mut Archetype {
        debug_assert!(self.archetype_table.contains_key(arche_id));
        unsafe { self.archetype_table.get_mut(arche_id).unwrap_unchecked() }
    }

    pub fn delete_entity(&mut self, entity: Entity, entity_manager: &mut EntityManager) {
        // SAFETY: Already carried out entity validation prior to calling this function.
        let entity_record = unsafe { entity_manager.get_record_unchecked(entity) };

        let arche = unsafe { self.get_mut_unchecked(&entity_record.archetype_id) };

        for storage in arche.components.values_mut() {
            unsafe { storage.delete_unchecked(entity_record.archetype_row) };
        }

        unsafe { arche.delete_entity(entity, entity_manager) };
    }

    /// # Safety
    /// - src_arche_id and dst_arche_id must be valid archetypes within this manager.
    pub unsafe fn insert_graph_edge(
        &mut self,
        src_arche_id: &ArchetypeID,
        dst_arche_id: &ArchetypeID,
        edge_comp_id: &ComponentID,
    ) {
        unsafe {
            self.get_mut_unchecked(src_arche_id)
                .edges
                .insert(edge_comp_id.clone(), dst_arche_id.clone())
        };
        unsafe {
            self.get_mut_unchecked(dst_arche_id)
                .edges
                .insert(edge_comp_id.clone(), src_arche_id.clone())
        };
    }

    /// There is a lot of unsafe code in this function, but it is all "safe" as long as the caller
    /// ensures that the entity is valid.
    pub fn add_component<T: Component>(
        &mut self,
        component: T,
        entity: Entity,
        comp_manager: &ComponentManager,
        entity_manager: &mut EntityManager,
    ) {
        let comp_id = comp_manager.get_id::<T>();

        // SAFETY: Already carried out entity validation prior to calling this function.
        let entity_record = unsafe { entity_manager.get_record_unchecked(entity) };

        let src_arche_id = &entity_record.archetype_id;
        let dst_arche_id =
            &self.get_extended_archetype(src_arche_id.clone(), comp_id, comp_manager);

        // SAFETY: Archetypes are guaranteed to be unique, so we can safely get mutable references
        let src_arche = unsafe { &mut *(self.get_mut_unchecked(src_arche_id) as *mut Archetype) };
        let dst_arche = unsafe { &mut *(self.get_mut_unchecked(dst_arche_id) as *mut Archetype) };

        // SAFETY: The destination archetype is guaranteed to have the component ID as it has
        //         been extended to include the component ID.
        unsafe { dst_arche.push_component(comp_id, component) };

        // TODO: this is dumb and temporary
        let comp_ids: Vec<ComponentID> = src_arche.comp_ids().map(|id| *id).collect();

        unsafe { src_arche.move_entity(entity, comp_ids.iter(), dst_arche, entity_manager) };
    }

    /// There is a lot of unsafe code in this function, but it is all "safe" as long as the caller
    /// ensures that the entity is valid.
    pub fn remove_component<T: Component>(
        &mut self,
        entity: Entity,
        comp_manager: &ComponentManager,
        entity_manager: &mut EntityManager,
    ) {
        let comp_id = comp_manager.get_id::<T>();

        // SAFETY: Already carried out entity validation prior to calling this function.
        let entity_record = unsafe { entity_manager.get_record_unchecked(entity) };

        let src_arche_id = &entity_record.archetype_id;
        let dst_arche_id = &self.get_reduced_archetype(src_arche_id.clone(), comp_id, comp_manager);

        // SAFETY: Archetypes are guaranteed to be unique, so we can safely get mutable references
        let src_arche = unsafe { &mut *(self.get_mut_unchecked(src_arche_id) as *mut Archetype) };
        let dst_arche = unsafe { &mut *(self.get_mut_unchecked(dst_arche_id) as *mut Archetype) };

        // SAFETY: The source archetype is guaranteed to have the component ID as it was checked, prior
        //         to calling this function, that the source archetype contains the component ID.
        unsafe { src_arche.delete_component(comp_id, entity_record.archetype_row) };

        // TODO: this is dumb and temporary
        let comp_ids: Vec<ComponentID> = dst_arche.comp_ids().map(|id| *id).collect();

        unsafe { src_arche.move_entity(entity, comp_ids.iter(), dst_arche, entity_manager) };
    }

    pub fn get_extended_archetype(
        &mut self,
        src_arche_id: ArchetypeID,
        new_comp_id: ComponentID,
        comp_manager: &ComponentManager,
    ) -> ArchetypeID {
        // archetype already has the edge to the archetype with the component
        if let Some(dst_arche_id) =
            unsafe { self.get_unchecked(&src_arche_id).edges.get(new_comp_id) }
        {
            return dst_arche_id.clone();
        }

        let target_arche_id = {
            let mut id = src_arche_id.clone();
            id.set(new_comp_id);
            id
        };

        // archetype with the component already exists in the graph, but there was no edge
        // from the src archetype, so add it
        if self.archetype_table.contains_key(&target_arche_id) {
            unsafe { self.insert_graph_edge(&src_arche_id, &target_arche_id, &new_comp_id) };
            return target_arche_id;
        }

        // archetype with the component did not exist, so create it
        let mut dst_arche = Archetype::new(target_arche_id.clone());

        // add the new component storage to the archetype
        dst_arche.components.insert(
            new_comp_id,
            ComponentStorage::from_metadata(new_comp_id, comp_manager.get_metadata(new_comp_id)),
        );

        // add the other components storages, inherited from the src archetype
        for comp_storage in unsafe { self.get_unchecked(&src_arche_id).components.values() } {
            dst_arche
                .components
                .insert(comp_storage.id, ComponentStorage::from_other(comp_storage));
        }

        self.new_archetypes_queue.push(dst_arche.id.clone());
        self.archetype_table.insert(dst_arche.id.clone(), dst_arche);
        unsafe { self.insert_graph_edge(&src_arche_id, &target_arche_id, &new_comp_id) };

        return target_arche_id;
    }

    pub fn get_reduced_archetype(
        &mut self,
        src_arche_id: ArchetypeID,
        old_comp_id: ComponentID,
        comp_manager: &ComponentManager,
    ) -> ArchetypeID {
        // archetype already has the edge to the archetype with the component
        if let Some(dst_arche_id) =
            unsafe { self.get_unchecked(&src_arche_id).edges.get(old_comp_id) }
        {
            return dst_arche_id.clone();
        }

        let target_arche_id = {
            let mut id = src_arche_id.clone();
            id.clear(old_comp_id);
            id
        };

        // archetype without the component already exists in the graph, but there was no edge
        // from the src archetype, so add it
        if self.archetype_table.contains_key(&target_arche_id) {
            unsafe { self.insert_graph_edge(&src_arche_id, &target_arche_id, &old_comp_id) };
            return target_arche_id;
        }

        // archetype with the component did not exist, so create it
        let mut dst_arche = Archetype::new(target_arche_id.clone());

        // add the components storages, inherited from the src archetype
        for comp_storage in unsafe { self.get_unchecked(&src_arche_id).components.values() } {
            if comp_storage.id == old_comp_id {
                continue;
            }

            dst_arche
                .components
                .insert(comp_storage.id, ComponentStorage::from_other(comp_storage));
        }

        self.new_archetypes_queue.push(dst_arche.id.clone());
        self.archetype_table.insert(dst_arche.id.clone(), dst_arche);
        unsafe { self.insert_graph_edge(&src_arche_id, &target_arche_id, &old_comp_id) };

        return target_arche_id;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn many_mut() {
        let mut manager = ArchetypeManager::new();

        let arche_1_id = BitSet::from_index(1);
        let arche_2_id = BitSet::from_index(2);

        let mut arche1 = Archetype::new(arche_1_id.clone());
        let arche2 = Archetype::new(arche_2_id.clone());

        arche1.entities.push(17);
        arche1.entities.push(32);

        manager.archetype_table.insert(arche_1_id.clone(), arche1);
        manager.archetype_table.insert(arche_2_id.clone(), arche2);

        let src_arche = unsafe { &mut *(manager.get_mut_unchecked(&arche_1_id) as *mut Archetype) };
        let dst_arche = unsafe { &mut *(manager.get_mut_unchecked(&arche_2_id) as *mut Archetype) };

        dst_arche.entities.push(src_arche.entities.swap_remove(0));
        src_arche.entities.push(dst_arche.entities.swap_remove(0));

        assert_eq!(src_arche.entities.len(), 2);
        assert_eq!(dst_arche.entities.len(), 0);
    }
}
