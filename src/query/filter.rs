use collections::BitSet;

use crate::{
    archetype::{Archetype, ArchetypeID, ArchetypeManager},
    component::ComponentID,
};

pub struct FilterBuilder {
    and: Vec<ComponentID>,
    not: Vec<ComponentID>,
    track: Vec<ComponentID>,
}

impl FilterBuilder {
    pub fn new() -> Self {
        Self {
            and: Vec::new(),
            not: Vec::new(),
            track: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            and: Vec::with_capacity(capacity),
            not: Vec::with_capacity(capacity),
            track: Vec::with_capacity(capacity),
        }
    }

    pub fn and(mut self, component: ComponentID) -> Self {
        self.and.push(component);
        self
    }

    pub fn not(mut self, component: ComponentID) -> Self {
        self.not.push(component);
        self
    }

    pub fn track(mut self, component: ComponentID) -> Self {
        self.track.push(component);
        self
    }

    pub fn build(self) -> Filter {
        let mut and_bitset = BitSet::new();
        for component in self.and.iter() {
            and_bitset.set(*component);
        }

        let mut not_bitset = BitSet::new();
        for component in self.not.iter() {
            not_bitset.set(*component);
        }

        Filter {
            and: self.and,
            not: self.not,
            track: self.track,

            and_bitset,
            not_bitset,
        }
    }
}

pub struct Filter {
    pub and: Vec<ComponentID>,
    pub not: Vec<ComponentID>,
    pub track: Vec<ComponentID>,

    pub and_bitset: BitSet,
    pub not_bitset: BitSet,
}

impl Filter {
    pub fn matches_archetype(&self, archetype: &mut Archetype) -> bool {
        let matches =
            archetype.id.contains(&self.and_bitset) && archetype.id.contains_none(&self.not_bitset);

        if matches {
            // Enable tracking for components that have opted in (via Tracked<T> parameter)
            for comp_id in self.track.iter() {
                unsafe { archetype.get_mut_storage(*comp_id).enable_tracking() };
            }
        }

        return matches;
    }

    pub fn matching_archetypes(
        &self,
        archetype_manager: &mut ArchetypeManager,
    ) -> Vec<ArchetypeID> {
        let mut matching = Vec::new();
        for archetype in archetype_manager.archetypes_mut() {
            if self.matches_archetype(archetype) {
                matching.push(archetype.id.clone());
            }
        }
        return matching;
    }
}

pub enum Tracked<T> {
    Modified(T),
    Unmodified(T),
}

impl<T> Tracked<T> {
    pub fn unwrap(self) -> T {
        match self {
            Self::Modified(t) => t,
            Self::Unmodified(t) => t,
        }
    }

    pub fn is_modified(&self) -> bool {
        match self {
            Self::Modified(_) => true,
            Self::Unmodified(_) => false,
        }
    }

    pub fn is_unmodified(&self) -> bool {
        match self {
            Self::Modified(_) => false,
            Self::Unmodified(_) => true,
        }
    }
}


pub struct And<T> {
    pub(crate) inner: T,
}

pub struct Not<T> {
    pub(crate) inner: T,
}