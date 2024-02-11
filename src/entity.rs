use collections::{Store, StoreKey};

use crate::archetype::ArchetypeID;

pub type Entity = u32;

#[derive(Default)]
pub struct EntityRecord {
    pub archetype_id: ArchetypeID,
    pub archetype_row: usize,
}

pub struct EntityManager {
    records: Store<EntityRecord>,
}

impl EntityManager {
    pub fn new() -> Self {
        EntityManager {
            records: Store::with_capacity(32),
        }
    }

    /// Returns the entity record at the index associated with this entity, regardless of whether
    /// the entity is alive or not.
    ///
    /// # Safety
    /// - Entity must be alive
    pub unsafe fn get_record(&self, entity: Entity) -> &EntityRecord {
        // SAFETY: If the entity is alive, the index is valid and the generation is current.
        unsafe { self.records.get_unchecked(StoreKey::from_key(entity)) }
    }

    /// Returns the entity record at the index associated with this entity, regardless of whether
    /// the entity is alive or not.
    ///
    /// # Safety
    /// - Entity must be alive
    pub unsafe fn get_record_mut_unchecked(&mut self, entity: Entity) -> &mut EntityRecord {
        // SAFETY: If the entity is alive, the index is valid and the generation is current.
        unsafe { self.records.get_mut_unchecked(StoreKey::from_key(entity)) }
    }

    pub fn create(&mut self) -> Entity {
        self.records.push(EntityRecord::default()).id()
    }

    pub fn alive(&self, entity: Entity) -> bool {
        self.records.contains_key(StoreKey::from_key(entity))
    }

    pub fn delete(&mut self, entity: Entity) {
        self.records.remove(StoreKey::from_key(entity));
    }
}
