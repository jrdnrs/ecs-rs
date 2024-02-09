use crate::archetype::{ArchetypeID, ArchetypeManager};

use super::bundle::ComponentBundle;

pub struct ComponentBundleIter<'w, 'q, C: ComponentBundle> {
    parameter_ids: &'q C::Id,
    archetype_manager: &'w ArchetypeManager,
    archetype_id_iter: core::slice::Iter<'q, ArchetypeID>,

    storages: Option<C::Storage<'w>>,
    index: usize,
    len: usize,
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

            storages: None,
            index: 0,
            len: 0,
        }
    }
}

impl<'w, 'q, C: ComponentBundle> Iterator for ComponentBundleIter<'w, 'q, C> {
    type Item = C::Item<'w>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.len {
            self.index = 0;

            let archetype_id = self.archetype_id_iter.next()?;

            // SAFETY:
            // - The archetype ID will definitely be valid as the iter was built using IDs from the
            //   archetype manager itself.
            let archetype = unsafe { self.archetype_manager.get_unchecked(archetype_id) };

            self.storages = Some(C::prepare_storage(archetype, self.parameter_ids));
            self.len = archetype.entities.len();

            return self.next();
        }

        // SAFETY:
        // - We can only reach this point if `self.storages` is `Some`
        let item = unsafe { Some(C::fetch_item(self.storages.unwrap_unchecked(), self.index)) };
        self.index += 1;

        return item;
    }
}
