use crate::{
    query::{
        bundle::{ComponentBundle, ResourceBundle},
        iter::ComponentBundleIter,
        Query,
    },
    World,
};

use super::{command::CommandQueue, schedule::Schedule};

pub struct SystemManager {
    schedules: Vec<Schedule>,
}

impl SystemManager {
    pub fn new() -> Self {
        Self {
            schedules: Vec::new(),
        }
    }

    pub fn add(&mut self, schedule: Schedule) {
        self.schedules.push(schedule);
    }

    pub fn update(&mut self, world: &mut World) {
        self.run_all(world);
        self.flush_commands(world);
        self.sync(world);

        world.archetype_manager.new_archetypes_queue.clear();
    }

    pub fn run_all(&mut self, world: &mut World) {
        for schedule in self.schedules.iter_mut() {
            schedule.run_all(world);
        }
    }

    fn flush_commands(&mut self, world: &mut World) {
        for schedule in self.schedules.iter_mut() {
            schedule.flush_commands(world);
        }
    }

    fn sync(&mut self, world: &mut World) {
        for schedule in self.schedules.iter_mut() {
            schedule.sync(world);
        }
    }
}

/// This is a bit ugly, but it basically represents a function that takes an iterator over
/// a bundle of components, a bundle of resources, and a command queue.
pub type SystemFn<C, R> =
    fn(ComponentBundleIter<'_, '_, C>, <R as ResourceBundle>::Item<'_>, &mut CommandQueue);

/// Every system has its own query that is used to fetch components and resources from the world. These
/// are then passed to the system function, along with the command queue from the [Schedule] which is a
/// parent of many systems.
///
/// The query is stored in the system, so that it can be updated when the world is updated.
pub struct System<C: ComponentBundle, R: ResourceBundle> {
    query: Query<C, R>,
    func: SystemFn<C, R>,
    last_update: u32,
}

impl<C: ComponentBundle, R: ResourceBundle> System<C, R> {
    pub fn new(query: Query<C, R>, func: SystemFn<C, R>) -> Self {
        Self {
            query,
            func,
            last_update: 0,
        }
    }

    pub fn into_schedule(self) -> Schedule {
        Schedule::new(vec![Box::new(self)], CommandQueue::new())
    }

    pub fn run(&mut self, command_buffer: &mut CommandQueue, world: &mut World) {
        let iter = self.query.iter(world);
        let resources = unsafe {
            R::fetch_item(
                &world.resource_manager.resources,
                self.query.res_param_ids,
            )
        };
        (self.func)(iter, resources, command_buffer);
        self.last_update = world.tick;
    }

    pub fn sync(&mut self, world: &mut World) {
        self.query.sync(world)
    }
}

pub trait AnySystem {
    fn run(&mut self, command_buffer: &mut CommandQueue, world: &mut World);
    fn sync(&mut self, world: &mut World);
}

impl<C: ComponentBundle, R: ResourceBundle> AnySystem for System<C, R> {
    fn run(&mut self, command_buffer: &mut CommandQueue, world: &mut World) {
        System::run(self, command_buffer, world)
    }

    fn sync(&mut self, world: &mut World) {
        System::sync(self, world)
    }
}

#[cfg(test)]
mod tests {
    use crate::{entity::Entity, query::filter::Tracked, system::schedule::ScheduleBuilder, World};

    use super::*;

    struct Speed {
        v: usize,
    }
    struct Health {
        v: usize,
    }
    struct Global {
        a: usize,
        b: usize,
        c: usize,
    }

    #[test]
    fn test_system() {
        let mut world = World::new();
        world.register_component::<Speed>();
        world.register_component::<Health>();

        let speed_system = System::new(
            world.query::<(Entity, &Speed)>().build(),
            |components, _, command_buffer| {
                for (e, s) in components {
                    println!("Speed: {}", s.v);
                    command_buffer.add_component(e, Health { v: e as usize });
                }
            },
        );

        for i in 0..10 {
            let player = world.create_entity();
            world.add_component(player, Speed { v: i });
        }

        world.add_schedule(ScheduleBuilder::new().add(speed_system).build());
        // This should run the speed system
        world.update();

        let health_system = System::new(
            world.query::<(Entity, &Health)>().build(),
            |components, _, command_buffer| {
                for (e, h) in components {
                    println!("Health: {}", h.v);
                    command_buffer.remove_entity(e);
                }
            },
        );

        world.add_schedule(ScheduleBuilder::new().add(health_system).build());
        // The speed system adds health components to all entities, so they should move to different archetypes.
        // Calling update should update the archetype ids of all systems, so the speed system should run again,
        // as well as the health system.
        world.update();

        // The health system removes all entities, so this should not run any systems.
        world.update();
    }

    #[test]
    fn tracking_test() {
        let mut world = World::new();
        world.register_component::<Speed>();
        world.register_component::<Health>();

        // BUG: Adding components to an entity, before creating the system that queries for them,
        //      means that the relevant archetypes will be tracked - great.
        //
        //      However, each newly created archetype will be added to a queue that is processed when the
        //      system manager flushes, as newly created archetypes may be added to existing queries if they match.
        //
        //      This means that the archetype will be added twice, as we are not using an actual set, but a vector.

        // BUG: A system that uses the `flag_modified` command, can fail if the component storage is not tracked.
        //      This can occur if another system, with identical queries but including a tracked version of
        //      the component, is not present which means tracking would not be added automatically.
        //
        //      I think the solution is just add a way to manually add tracking? This might be tricky. An easier way
        //      would be to require the `Tracked` enum by required for the `flag_modified` command.

        let flag_modify_system = System::new(
            world.query::<(Entity, &Speed)>().build(),
            |components, _, command_buffer| {
                for (e, s) in components {
                    println!("Speed: {}", s.v);
                    if s.v % 2 == 0 {
                        command_buffer.flag_modified::<Speed>(e);
                    }
                }
            },
        );

        let tracked_system = System::new(
            world.query::<Tracked<&Speed>>().build(),
            |components, _, _| {
                for s in components {
                    println!("Modified: {}", if s.is_modified() { "yes" } else { "no" });
                }
            },
        );

        for i in 0..10 {
            let player = world.create_entity();
            world.add_component(player, Speed { v: i });
        }

        world.add_schedule(
            ScheduleBuilder::new()
                .add(flag_modify_system)
                .add(tracked_system)
                .build(),
        );

        // First update will run nothing, but the flush thereafter will update the relevant archetypes
        // for the system, so the next update will run the system.
        println!("First update");
        world.update();

        // Systems run, and all newly added components are counted as modified
        println!("Second update");
        world.update();

        // Systems run, but as the flag_modify_system only modifies some components, the tracked system
        // will reflect this
        println!("Third update");
        world.update();
    }

    #[test]
    fn resource_test() {
        let mut world = World::new();
        world.register_component::<Speed>();
        world.register_component::<Health>();

        let global = Global { a: 3, b: 5, c: 7 };
        let global_id = world.add_resource(global);

        let global = unsafe { world.resource_manager.get_unchecked::<Global>(global_id) };
        assert_eq!(global.a, 3);
        assert_eq!(global.b, 5);
        assert_eq!(global.c, 7);

        let system_with_resource = System::new(
            world
                .query::<(&Health, &Speed)>()
                .with_resources::<&mut Global>()
                .build(),
            |iter, global, _| {
                global.a = 4;
                global.b = 6;
                global.c = 8;

                for (h, s) in iter {
                    println!("{}", h.v);

                    assert_eq!(global.a, 4);
                    assert_eq!(global.b, 6);
                    assert_eq!(global.c, 8);
                }
            },
        );

        world.add_schedule(ScheduleBuilder::new().add(system_with_resource).build());

        for i in 0..10 {
            let player = world.create_entity();
            world.add_component(player, Speed { v: i });
            world.add_component(player, Health { v: i });
        }

        // First update will run nothing, but the flush thereafter will update the relevant archetypes for the system,
        // so the next update will run the system.
        world.update();
        world.update();

        let global = unsafe { world.resource_manager.get_unchecked::<Global>(global_id) };
        assert_eq!(global.a, 4);
        assert_eq!(global.b, 6);
        assert_eq!(global.c, 8);
    }
}
