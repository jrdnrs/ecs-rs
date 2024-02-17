use std::collections::HashMap;

use collections::{BitSet, SparseMap};

use crate::{
    component::{storage::ComponentStorage, Component, ComponentID, ComponentManager},
    entity::{Entity, EntityManager},
    util::get_two_mut_unchecked,
    ComponentBundle,
};

/// Unique sequential integer
pub type ArchetypeID = usize;

pub struct Archetype {
    pub id: ArchetypeID,

    /// A bitset that represents the component IDs that are present within this archetype,
    /// where the index of each set bit corresponds to the component ID.
    pub component_id_bitset: BitSet,

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
    pub fn new(id: ArchetypeID, comp_ids: BitSet) -> Self {
        Self {
            id,
            component_id_bitset: comp_ids,
            edges: SparseMap::with_capacity(4),
            components: SparseMap::with_capacity(4),
            entities: Vec::with_capacity(8),
        }
    }

    pub fn comp_ids(&self) -> &[ComponentID] {
        self.components.keys()
    }

    pub fn has_component(&self, comp_id: ComponentID) -> bool {
        self.component_id_bitset.test(comp_id)
    }

    /// # Safety
    /// - The entity must be alive, and does not already exist in this archetype.
    pub unsafe fn push_entity(&mut self, entity: Entity, entity_manager: &mut EntityManager) {
        // SAFETY: Caller ensures that the entity is alive.
        let entity_record = unsafe { entity_manager.get_record_mut_unchecked(entity) };

        entity_record.archetype_row = self.entities.len();
        entity_record.archetype_id = self.id.clone();
        self.entities.push(entity);
    }

    /// # Safety
    /// - The entity must be alive, and only exists in this archetype.
    pub unsafe fn delete_entity(&mut self, entity: Entity, entity_manager: &mut EntityManager) {
        // SAFETY: Caller ensures that the entity is alive.
        let entity_row = unsafe { entity_manager.get_record(entity).archetype_row };

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
        unsafe { storage.delete(row) }
    }

    /// # Safety
    /// - The component ID must exist within this archetype, as no bounds checking is performed.
    /// - The row must be within the bounds of the underlying vec.
    pub unsafe fn get_component<T: Component>(&self, comp_id: ComponentID, row: usize) -> &T {
        // SAFETY: Deferred to the caller
        let storage = unsafe { self.get_storage(comp_id) };
        unsafe { storage.get(row) }
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
        unsafe { storage.get_mut(row) }
    }

    /// # Safety
    /// - The component ID must exist within this archetype as well as the destination archetype, as no bounds
    ///   checking is performed.
    /// - The `src_row` must be within the bounds of the underlying vec.
    pub unsafe fn transfer_component(
        &mut self,
        comp_id: ComponentID,
        src_row: usize,
        dst_arche: &mut Self,
    ) {
        // SAFETY: Deferred to the caller
        let src_storage = unsafe { self.get_mut_storage(comp_id) };
        let dst_storage = unsafe { dst_arche.get_mut_storage(comp_id) };

        unsafe { src_storage.transfer(src_row, dst_storage) }
    }

    /// # Safety
    /// - The component IDs must exist within this archetype as well as the destination archetype, as no bounds
    ///   checking is performed.
    /// - The entity must be alive, and only exists in this archetype.
    pub unsafe fn transfer_entity<'a>(
        &mut self,
        entity: Entity,
        comp_ids: impl Iterator<Item = ComponentID>,
        dst_arche: &mut Archetype,
        entity_manager: &mut EntityManager,
    ) {
        // SAFETY: Caller ensures that the entity is alive.
        let entity_record = unsafe { entity_manager.get_record(entity) };

        for comp_id in comp_ids {
            // SAFETY:
            // - Caller ensures component ID is valid for both archetypes.
            // - Entity is alive, so archetype_row is assumed to be valid
            unsafe { self.transfer_component(comp_id, entity_record.archetype_row, dst_arche) };
        }

        // SAFETY: Entity is alive and exists within this archetype
        unsafe { self.delete_entity(entity, entity_manager) };
        // SAFETY: Entity is alive and does not exist within this destination archetype
        unsafe { dst_arche.push_entity(entity, entity_manager) };
    }

    /// # Safety
    /// - The component ID must exist within this archetype, as no bounds checking is performed.
    pub unsafe fn get_storage(&self, comp_id: ComponentID) -> &ComponentStorage {
        debug_assert!(
            self.component_id_bitset.test(comp_id),
            "Component ID does not match archetype"
        );

        // SAFETY: Caller ensures that the component ID exists within this archetype, so this key is
        //         valid, and we do not remove components from archetypes so the underlying index is
        //         guaranteed to be valid as well.
        unsafe { self.components.get_unchecked(comp_id) }
    }

    /// # Safety
    /// - The component ID must exist within this archetype, as no bounds checking is performed.
    pub unsafe fn get_mut_storage(&mut self, comp_id: ComponentID) -> &mut ComponentStorage {
        debug_assert!(
            self.component_id_bitset.test(comp_id),
            "Component ID does not match archetype"
        );

        // SAFETY: Caller ensures that the component ID exists within this archetype, so this key is
        //         valid, and we do not remove components from archetypes so the underlying index is
        //         guaranteed to be valid as well.
        unsafe { self.components.get_mut_unchecked(comp_id) }
    }
}

pub struct ArchetypeManager {
    /// A map of bitsets to archetype IDs. The bitset represents the component IDs that are present
    ids: HashMap<BitSet, ArchetypeID, ahash::RandomState>,

    /// A table of all archetypes that exist within the world.
    pub(crate) archetype_table: Vec<Archetype>,

    /// When a query is first created, archetypes relevant to that query are cached. If a new archetype
    /// is created it is added to this queue so that, after all systems have run for a given world update,
    /// they can be checked for relevance to every query and added to their cache, before being cleared.
    pub(crate) new_archetypes_queue: Vec<ArchetypeID>,
}

impl ArchetypeManager {
    pub fn new() -> Self {
        let ids = HashMap::with_capacity_and_hasher(8, ahash::RandomState::default());

        // Includes root archetype
        let archetype_table = vec![Archetype::new(0, BitSet::new())];

        Self {
            ids,
            archetype_table,
            new_archetypes_queue: Vec::new(),
        }
    }

    /// Creates a new archetype with the given component IDs
    ///
    /// The archetype should not already exist, as no check is performed to ensure that it does not.
    pub fn create_archetype(&mut self, comp_ids: BitSet) -> ArchetypeID {
        debug_assert!(
            !self.ids.contains_key(&comp_ids),
            "Archetype with the given component IDs already exists"
        );

        let arche_id = self.archetype_table.len();
        let arche = Archetype::new(arche_id, comp_ids.clone());
        self.archetype_table.push(arche);
        self.ids.insert(comp_ids, arche_id);
        self.new_archetypes_queue.push(arche_id);

        arche_id
    }

    pub fn get_root(&self) -> &Archetype {
        // SAFETY: The root archetype is always present
        unsafe { self.archetype_table.get_unchecked(0) }
    }

    pub fn get_root_mut(&mut self) -> &mut Archetype {
        // SAFETY: The root archetype is always present
        unsafe { self.archetype_table.get_unchecked_mut(0) }
    }

    /// # Safety
    /// - The archetype ID must exist within this manager, as no existence check is performed.
    pub unsafe fn get(&self, arche_id: ArchetypeID) -> &Archetype {
        debug_assert!(arche_id < self.archetype_table.len());
        unsafe { self.archetype_table.get_unchecked(arche_id) }
    }

    /// # Safety
    /// - The archetype ID must exist within this manager, as no existence check is performed.
    pub unsafe fn get_mut(&mut self, arche_id: ArchetypeID) -> &mut Archetype {
        debug_assert!(arche_id < self.archetype_table.len());
        unsafe { self.archetype_table.get_unchecked_mut(arche_id) }
    }

    /// # Safety
    /// - The entity must be alive.
    pub unsafe fn delete_entity(&mut self, entity: Entity, entity_manager: &mut EntityManager) {
        // SAFETY: Caller ensures that the entity is alive.
        let entity_record = unsafe { entity_manager.get_record(entity) };

        // SAFETY: Entity is alive, so archetype_id is valid as it was copied from the archetype, and
        //         we do not delete archetypes.
        let arche = unsafe { self.get_mut(entity_record.archetype_id) };

        for storage in arche.components.values_mut() {
            // SAFETY: Entity is alive, so archetype_row is assumed to be valid
            unsafe { storage.delete(entity_record.archetype_row) };
        }

        // SAFETY: Entity is alive
        unsafe { arche.delete_entity(entity, entity_manager) };
    }

    /// # Safety
    /// - src_arche_id and dst_arche_id must be valid archetypes within this manager.
    pub unsafe fn insert_graph_edge(
        &mut self,
        src_arche_id: ArchetypeID,
        dst_arche_id: ArchetypeID,
        edge_comp_id: ComponentID,
    ) {
        // SAFETY: Caller ensure `src_arche_id` exists within this manager.
        let src_arche = unsafe { self.get_mut(src_arche_id) };
        src_arche.edges.insert(edge_comp_id, dst_arche_id);

        // SAFETY: Caller ensure `dst_arche_id` exists within this manager.
        let dst_arche = unsafe { self.get_mut(dst_arche_id) };
        dst_arche.edges.insert(edge_comp_id, src_arche_id);
    }

    /// # Safety
    /// - The entity must be alive.
    ///
    /// # Panics
    /// - If the component has not been registered with the component manager.
    pub unsafe fn add_component<T: Component>(
        &mut self,
        component: T,
        entity: Entity,
        comp_manager: &ComponentManager,
        entity_manager: &mut EntityManager,
    ) {
        let comp_id = comp_manager.get_id::<T>();

        // SAFETY: Caller ensures that the entity is alive
        let entity_record = unsafe { entity_manager.get_record(entity) };

        let src_arche_id = entity_record.archetype_id;

        // SAFETY: `src_arche_id`, as retrieved from the entity record, is guaranteed to be valid
        //        as it was copied from the archetype itself, and we do not delete archetypes.
        let dst_arche_id =
            unsafe { self.get_extended_archetype(src_arche_id, comp_id, comp_manager) };

        // SAFETY: Archetypes are guaranteed to exist and be unique, so we can safely get mutable references
        let (src_arche, dst_arche) =
            unsafe { get_two_mut_unchecked(&mut self.archetype_table, src_arche_id, dst_arche_id) };

        // SAFETY: The destination archetype is guaranteed to have the component ID as it has
        //         been extended to include the component ID.
        unsafe { dst_arche.push_component(comp_id, component) };

        // HACK: Get around borrow checker by redefining slice with different lifetime, until I find a
        //       better way to do this. These component IDs are read from a different part of the archetype
        //       than we are going to mutate, so it should be safe.
        let comp_ids = {
            // SAFETY: The slice is just being redefined with a different lifetime which is ok as we are
            //         not actually modifying the underlying data.
            let comp_id_slice = unsafe {
                core::slice::from_raw_parts(
                    src_arche.comp_ids().as_ptr(),
                    src_arche.comp_ids().len(),
                )
            };
            comp_id_slice.iter().copied()
        };

        // SAFETY:
        // - The entity is alive, and only exists in the source archetype.
        // - The source archetype is guaranteed to have the component IDs as we source the
        //   component IDs from the source archetype.
        // - As we are adding a component, in moving to the destination archetype, the destination
        //   archetype will have the component IDs of the source archetype.
        unsafe { src_arche.transfer_entity(entity, comp_ids, dst_arche, entity_manager) };
    }

    /// # Safety
    /// - The entity must be alive.
    ///
    /// # Panics
    /// - If the component has not been registered with the component manager.
    pub unsafe fn remove_component<T: Component>(
        &mut self,
        entity: Entity,
        comp_manager: &ComponentManager,
        entity_manager: &mut EntityManager,
    ) {
        let comp_id = comp_manager.get_id::<T>();

        // SAFETY: Already carried out entity validation prior to calling this function.
        let entity_record = unsafe { entity_manager.get_record(entity) };

        let src_arche_id = entity_record.archetype_id;

        // SAFETY: `src_arche_id`, as retrieved from the entity record, is guaranteed to be valid
        //        as it was copied from the archetype itself, and we do not delete archetypes.
        let dst_arche_id =
            unsafe { self.get_reduced_archetype(src_arche_id, comp_id, comp_manager) };

        // SAFETY: Archetypes are guaranteed to exist and be unique, so we can safely get mutable references
        let (src_arche, dst_arche) =
            unsafe { get_two_mut_unchecked(&mut self.archetype_table, src_arche_id, dst_arche_id) };

        // SAFETY: The source archetype is guaranteed to have the component ID as it has
        //         been reduced to exclude the component ID.
        unsafe { src_arche.delete_component(comp_id, entity_record.archetype_row) };

        // HACK: Get around borrow checker by redefining slice with different lifetime, until I find a
        //       better way to do this. These component IDs are read from a different part of the archetype
        //       than we are going to mutate, so it should be safe.
        let comp_ids = {
            // SAFETY: The slice is just being redefined with a different lifetime which is ok as we are
            //         not actually modifying the underlying data.
            let comp_id_slice = unsafe {
                core::slice::from_raw_parts(
                    dst_arche.comp_ids().as_ptr(),
                    dst_arche.comp_ids().len(),
                )
            };
            comp_id_slice.iter().copied()
        };

        // SAFETY:
        // - The entity is alive, and only exists in the source archetype.
        // - The destination archetype is guaranteed to have the component IDs as we source the
        //   component IDs from the destination archetype.
        // - As we are removing a component, in moving to the destination archetype, the source
        //   archetype will have the component IDs of the destination archetype.
        unsafe { src_arche.transfer_entity(entity, comp_ids, dst_arche, entity_manager) };
    }

    /// # Safety
    /// - `src_arche_id` must be a valid archetype within this manager.
    pub unsafe fn get_extended_archetype(
        &mut self,
        src_arche_id: ArchetypeID,
        new_comp_id: ComponentID,
        comp_manager: &ComponentManager,
    ) -> ArchetypeID {
        let src_arche = unsafe { self.get(src_arche_id) };

        if let Some(&dst_arche_id) = src_arche.edges.get(new_comp_id) {
            // Archetype already has the edge to the archetype with the component!
            return dst_arche_id;
        }

        let target_comp_bitset = {
            let mut bitset = src_arche.component_id_bitset.clone();
            bitset.set(new_comp_id);
            bitset
        };

        if let Some(&dst_arche_id) = self.ids.get(&target_comp_bitset) {
            // Archetype with the component already existed in the graph, but there was no edge
            // from the src archetype, so add it for future use
            unsafe { self.insert_graph_edge(src_arche_id, dst_arche_id, new_comp_id) };
            return dst_arche_id;
        }

        // Archetype with the component did not exist, so create it
        let dst_arche_id = self.create_archetype(target_comp_bitset);
        // SAFETY: Archetypes are guaranteed to exist and be unique, so we can safely get mutable references
        let (src_arche, dst_arche) =
            unsafe { get_two_mut_unchecked(&mut self.archetype_table, src_arche_id, dst_arche_id) };

        // add the new component storage to the archetype
        dst_arche.components.insert(
            new_comp_id,
            ComponentStorage::from_metadata(new_comp_id, comp_manager.get_metadata(new_comp_id)),
        );

        // add the other components storages, inherited from the src archetype
        for comp_storage in src_arche.components.values() {
            dst_arche.components.insert(
                comp_storage.id(),
                ComponentStorage::from_other(comp_storage),
            );
        }

        unsafe { self.insert_graph_edge(src_arche_id, dst_arche_id, new_comp_id) };

        dst_arche_id
    }

    /// # Safety
    /// - `src_arche_id` must be a valid archetype within this manager.
    pub unsafe fn get_reduced_archetype(
        &mut self,
        src_arche_id: ArchetypeID,
        old_comp_id: ComponentID,
        comp_manager: &ComponentManager,
    ) -> ArchetypeID {
        let src_arche = unsafe { self.get(src_arche_id) };

        if let Some(&dst_arche_id) = src_arche.edges.get(old_comp_id) {
            // Archetype already has the edge to the archetype without the component!
            return dst_arche_id;
        }

        let target_comp_bitset = {
            let mut bitset = src_arche.component_id_bitset.clone();
            bitset.clear(old_comp_id);
            bitset
        };

        if let Some(&dst_arche_id) = self.ids.get(&target_comp_bitset) {
            // Archetype without the component already existed in the graph, but there was no edge
            // from the src archetype, so add it for future use
            unsafe { self.insert_graph_edge(src_arche_id, dst_arche_id, old_comp_id) };
            return dst_arche_id;
        }

        // Archetype without the component did not exist, so create it
        let dst_arche_id = self.create_archetype(target_comp_bitset);
        // SAFETY: Archetypes are guaranteed to exist and be unique, so we can safely get mutable references
        let (src_arche, dst_arche) =
            unsafe { get_two_mut_unchecked(&mut self.archetype_table, src_arche_id, dst_arche_id) };

        // add the components storages, inherited from the src archetype (except the one to remove)
        for comp_storage in src_arche.components.values() {
            if comp_storage.id() == old_comp_id {
                continue;
            }

            dst_arche.components.insert(
                comp_storage.id(),
                ComponentStorage::from_other(comp_storage),
            );
        }

        unsafe { self.insert_graph_edge(src_arche_id, dst_arche_id, old_comp_id) };

        dst_arche_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
