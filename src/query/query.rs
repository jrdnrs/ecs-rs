use core::marker::PhantomData;

use crate::{
    archetype::{ArchetypeID, ArchetypeManager},
    component::ComponentManager,
    resource::ResourceManager,
    system::{System, SystemFn},
    World,
};

use super::{
    bundle::{ComponentBundle, FilterBundle, ResourceBundle},
    filter::{Filter, FilterBuilder},
    iter::ComponentBundleIter,
};

pub struct QueryBuilder<'w, T> {
    component_manager: &'w ComponentManager,
    resource_manager: &'w ResourceManager,
    archetype_manager: &'w mut ArchetypeManager,
    filter_builder: FilterBuilder,
    _marker: PhantomData<T>,
}

// Without Resources
impl<'w, C: ComponentBundle> QueryBuilder<'w, (C,)> {
    pub fn new(
        component_manager: &'w ComponentManager,
        resource_manager: &'w ResourceManager,
        archetype_manager: &'w mut ArchetypeManager,
    ) -> Self {
        QueryBuilder {
            component_manager,
            resource_manager,
            archetype_manager,
            filter_builder: FilterBuilder::with_capacity(C::count()),
            _marker: PhantomData,
        }
    }

    pub fn filter<CFilter: FilterBundle>(mut self) -> QueryBuilder<'w, (C,)> {
        let parameter_ids = CFilter::parameter_ids(&self.component_manager);
        self.filter_builder = CFilter::build_filter(self.filter_builder, &parameter_ids);

        self
    }

    pub fn with_resources<R: ResourceBundle>(self) -> QueryBuilder<'w, (C, R)> {
        QueryBuilder::<(C, R)>::new(
            self.component_manager,
            self.resource_manager,
            self.archetype_manager,
            self.filter_builder,
        )
    }

    pub fn build(self) -> Query<C, ()> {
        Query::new(
            self.component_manager,
            self.resource_manager,
            self.archetype_manager,
            self.filter_builder,
        )
    }
}

// With Resources
impl<'w, C: ComponentBundle, R: ResourceBundle> QueryBuilder<'w, (C, R)> {
    pub fn new(
        component_manager: &'w ComponentManager,
        resource_manager: &'w ResourceManager,
        archetype_manager: &'w mut ArchetypeManager,
        filter_builder: FilterBuilder,
    ) -> Self {
        QueryBuilder {
            component_manager,
            resource_manager,
            archetype_manager,
            filter_builder,
            _marker: PhantomData,
        }
    }

    pub fn filter<CFilter: FilterBundle>(mut self) -> QueryBuilder<'w, (C, R)> {
        let parameter_ids = CFilter::parameter_ids(&self.component_manager);
        self.filter_builder = CFilter::build_filter(self.filter_builder, &parameter_ids);

        self
    }

    pub fn build(self) -> Query<C, R> {
        Query::new(
            self.component_manager,
            self.resource_manager,
            self.archetype_manager,
            self.filter_builder,
        )
    }
}

/// A Query defines a set of components and resources that a system will operate on.
///
/// # Implementation
/// It records the archetype IDs that match the query, and provides an iterator over the
/// relevant component bundles from those archetypes. It also provides a method to sync
/// the query with the world, updating the archetype IDs, to account for any new archetypes
/// that have been created since the last sync.
pub struct Query<C: ComponentBundle, R: ResourceBundle> {
    pub(crate) comp_param_ids: C::Id,
    pub(crate) res_param_ids: R::Id,
    pub(crate) archetype_ids: Vec<ArchetypeID>,
    pub(crate) filter: Filter,
}

impl<'w, C: ComponentBundle, R: ResourceBundle> Query<C, R> {
    pub fn new(
        component_manager: &ComponentManager,
        resource_manager: &ResourceManager,
        archetype_manager: &mut ArchetypeManager,
        filter_builder: FilterBuilder,
    ) -> Self {
        let comp_param_ids = C::parameter_ids(component_manager);
        let res_param_ids = R::parameter_ids(resource_manager);
        let filter = C::build_filter(filter_builder, &comp_param_ids).build();
        let archetype_ids = filter.matching_archetypes(archetype_manager);

        Self {
            comp_param_ids,
            res_param_ids,
            archetype_ids,
            filter,
        }
    }

    pub fn into_system(self, system_fn: SystemFn<C, R>) -> System<C, R> {
        System::new(self, system_fn)
    }

    pub fn iter(&self, world: &'w World) -> ComponentBundleIter<'w, '_, C> {
        ComponentBundleIter::<'w, '_, C>::new(
            &world.archetype_manager,
            &self.comp_param_ids,
            &self.archetype_ids,
        )
    }

    pub fn sync(&mut self, world: &mut World) {
        self.update_archetype_ids(&mut world.archetype_manager);
        self.update_storage_trackers(&mut world.archetype_manager, world.tick);
    }

    fn update_archetype_ids(&mut self, archetype_manager: &mut ArchetypeManager) {
        for &arche_id in archetype_manager.new_archetypes_queue.iter() {
            let mut archetype = unsafe {
                archetype_manager
                    .archetype_table
                    .get_unchecked_mut(arche_id)
            };
            if self.filter.matches_archetype(&mut archetype) {
                self.archetype_ids.push(archetype.id);
            }
        }
    }

    fn update_storage_trackers(&mut self, archetype_manager: &mut ArchetypeManager, tick: u32) {
        // this updates the last_read of all tracked components
        for &arche_id in self.archetype_ids.iter() {
            for &comp_id in self.filter.track.iter() {
                unsafe {
                    archetype_manager
                        .get_mut(arche_id)
                        .get_mut_storage(comp_id)
                        .get_mut_tracker()
                        .last_read = tick
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{entity::Entity, And, World};

    struct Speed {
        v: usize,
    }
    struct Health {
        v: usize,
    }

    struct Power {
        v: usize,
    }

    struct Super;

    #[test]
    fn test_iter() {
        let mut world = World::new();
        world.register_component::<Speed>();
        world.register_component::<Health>();
        world.register_component::<Power>();

        for i in 0..1_000 {
            let player = world.create_entity();
            world.add_component(player, Speed { v: i });
            world.add_component(player, Health { v: i * 2 });
        }

        for i in 0..700 {
            let player = world.create_entity();
            world.add_component(player, Speed { v: i });
            world.add_component(player, Health { v: i * 2 });
            world.add_component(player, Power { v: i * 5 });

            if i % 2 == 0 {
                world.remove_component::<Speed>(player);
                assert!(!world.has_component::<Speed>(player));
                assert!(world.has_component::<Health>(player));
                assert!(world.has_component::<Power>(player));
            }

            if i % 5 == 0 {
                world.delete_entity(player);
                assert!(!world.is_entity_alive(player));
            }
        }

        let query = world.query::<(&mut Speed, &mut Health)>().build();

        assert_eq!(query.archetype_ids.len(), 2);

        let now = std::time::Instant::now();

        for (s, h) in query.iter(&mut world) {
            s.v += 3;
            h.v *= s.v;
        }

        println!("time: {:?}", now.elapsed());

        let query = world.query::<(Entity, &mut Health)>().build();

        let now = std::time::Instant::now();

        for (entity, h) in query.iter(&mut world) {
            // println!("{}", entity & (1 << 22) - 1);
            h.v *= 3;
        }

        println!("time: {:?}", now.elapsed());
    }

    #[test]
    fn option_test() {
        let mut world = World::new();
        world.register_component::<Speed>();
        world.register_component::<Health>();
        world.register_component::<Power>();

        for i in 0..1_000 {
            let player = world.create_entity();
            world.add_component(player, Speed { v: i });
            world.add_component(player, Health { v: i });
        }

        for i in 0..1_000 {
            let player = world.create_entity();
            world.add_component(player, Speed { v: i });
            world.add_component(player, Health { v: i });
            world.add_component(player, Power { v: i });
        }

        let query = world.query::<(&mut Speed, Option<&mut Power>)>().build();

        let now = std::time::Instant::now();
        for (s, h) in query.iter(&mut world) {
            if let Some(h) = h {
                s.v += h.v;
            } else {
                s.v += 1;
            }
        }
        println!("time: {:?}", now.elapsed());

        // This doesn't use an optional component, but it's just to compare the performance
        let query = world.query::<(&mut Speed, &mut Health)>().build();

        let now = std::time::Instant::now();
        for (s, h) in query.iter(&mut world) {
            s.v *= h.v;
        }
        println!("time: {:?}", now.elapsed());
    }

    #[test]
    fn and_tag_component() {
        let mut world = World::new();
        world.register_component::<Speed>();
        world.register_component::<Health>();
        world.register_component::<Super>();

        for i in 0..42 {
            let player = world.create_entity();
            world.add_component(player, Speed { v: i });
            world.add_component(player, Health { v: i });
        }

        for i in 0..77 {
            let player = world.create_entity();
            world.add_component(player, Speed { v: i });
            world.add_component(player, Health { v: i });
            world.add_component(player, Super);
        }

        // We only want 2 components in the iterator, but we want to filter archetypes
        // by those with all three components
        let query = world
            .query::<(&mut Speed, &mut Health)>()
            .filter::<And<Super>>()
            .build();

        let now = std::time::Instant::now();

        let mut count = 0;
        for (s, h) in query.iter(&mut world) {
            // println!("s: {}, h: {}, p: {}", s.v, h.v, p.v);
            count += 1;
        }

        assert_eq!(count, 77);

        println!("time: {:?}", now.elapsed());
    }
}
