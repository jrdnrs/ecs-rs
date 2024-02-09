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

    /// Returns the entity record for the given entity if it is still alive
    pub fn get_record(&self, entity: Entity) -> Option<&EntityRecord> {
        self.records.get(StoreKey::from_key(entity))
    }

    /// Returns the entity record at the index associated with this entity, regardless of whether
    /// the entity is alive or not.
    ///
    /// # Safety
    /// - No underlying bounds check is performed for the index associated with the given entity
    /// - No check is performed to ensure the generation of the entity matches the generation of the
    /// record at the given index
    pub unsafe fn get_record_unchecked(&self, entity: Entity) -> &EntityRecord {
        // SAFETY: Deferred to the caller
        unsafe { self.records.get_unchecked(StoreKey::from_key(entity)) }
    }

    /// Returns the entity record for the given entity if it is still alive
    pub fn get_record_mut(&mut self, entity: Entity) -> Option<&mut EntityRecord> {
        self.records.get_mut(StoreKey::from_key(entity))
    }

    /// Returns the entity record at the index associated with this entity, regardless of whether
    /// the entity is alive or not.
    ///
    /// # Safety
    /// - No underlying bounds check is performed for the index associated with the given entity
    /// - No check is performed to ensure the generation of the entity matches the generation of the
    /// record at the given index
    pub unsafe fn get_record_mut_unchecked(&mut self, entity: Entity) -> &mut EntityRecord {
        // SAFETY: Deferred to the caller
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
