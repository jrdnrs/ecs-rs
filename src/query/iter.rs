use crate::archetype::{ArchetypeID, ArchetypeManager};

use super::bundle::ComponentBundle;

pub struct ComponentBundleIter<'w, 'q, C: ComponentBundle> {
    parameter_ids: &'q C::Id,
    archetype_manager: &'w ArchetypeManager,
    archetype_id_iter: core::slice::Iter<'q, ArchetypeID>,

    chunk_iter: Option<ComponentChunkIter<'w, C>>,
}

impl<'w, 'q, C: ComponentBundle> ComponentBundleIter<'w, 'q, C> {
    pub fn new(
        archetype_manager: &'w ArchetypeManager,
        parameter_ids: &'q C::Id,
        archetype_ids: &'q [ArchetypeID],
    ) -> Self {
        Self {
            archetype_manager,
            parameter_ids,
            archetype_id_iter: archetype_ids.iter(),

            chunk_iter: None,
        }
    }

    fn next_chunk(&mut self) -> Option<ComponentChunkIter<'w, C>> {
        let archetype_id = self.archetype_id_iter.next()?;

        // SAFETY:
        // - The archetype ID will definitely be valid as the iter was built using IDs from the
        //   archetype manager itself.
        let archetype = unsafe { self.archetype_manager.get(*archetype_id) };

        Some(ComponentChunkIter::new(
            C::prepare_storage(archetype, self.parameter_ids),
            archetype.entities.len(),
        ))
    }
}

impl<'w, 'q, C: ComponentBundle> Iterator for ComponentBundleIter<'w, 'q, C> {
    type Item = C::Item<'w>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.chunk_iter.is_none() {
                match self.next_chunk() {
                    Some(chunk_iter) => self.chunk_iter = Some(chunk_iter),
                    None => return None,
                }
            }

            match unsafe { self.chunk_iter.as_mut().unwrap_unchecked().next() } {
                Some(item) => return Some(item),
                None => self.chunk_iter = None,
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self
            .archetype_id_iter
            .clone()
            .map(|id| unsafe { self.archetype_manager.get(*id).entities.len() })
            .sum::<usize>()
            + self.chunk_iter.as_ref().map(|iter| iter.len - iter.index).unwrap_or(0);

        (remaining, Some(remaining))
    }
}

pub struct ComponentChunkIter<'w, C: ComponentBundle> {
    storages: C::Storage<'w>,
    index: usize,
    len: usize,
}

impl<'w, C: ComponentBundle> ComponentChunkIter<'w, C> {
    pub fn new(storages: C::Storage<'w>, len: usize) -> Self {
        Self {
            storages,
            index: 0,
            len,
        }
    }
}

impl<'w, C: ComponentBundle> Iterator for ComponentChunkIter<'w, C> {
    type Item = C::Item<'w>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.len {
            return None;
        }

        let item = unsafe { C::fetch_item(self.storages, self.index) };
        self.index += 1;

        Some(item)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len - self.index, Some(self.len - self.index))
    }
}
