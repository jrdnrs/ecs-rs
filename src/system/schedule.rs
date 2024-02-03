use crate::{
    query::bundle::{ComponentBundle, ResourceBundle},
    World,
};

use super::{command::CommandQueue, AnySystem, System};

/// A builder for [Schedule]s
///
/// # Implementation
/// Unlike with commands in [CommandQueue]s, here we store systems as heap allocated trait objects as there
/// are fewer of them and are expected to stick around unlike commands which are ephemeral.
///
pub struct ScheduleBuilder {
    systems: Vec<Box<dyn AnySystem>>,
}

impl ScheduleBuilder {
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
        }
    }

    pub fn add<C: ComponentBundle, R: ResourceBundle>(mut self, system: System<C, R>) -> Self {
        self.systems.push(Box::new(system));
        self
    }

    pub fn build(self) -> Schedule {
        Schedule::new(self.systems, CommandQueue::new())
    }
}

pub struct Schedule {
    systems: Vec<Box<dyn AnySystem>>,
    commands: CommandQueue,
}

impl Schedule {
    pub fn new(systems: Vec<Box<dyn AnySystem>>, commands: CommandQueue) -> Self {
        Self { systems, commands }
    }

    pub fn add<C: ComponentBundle, R: ResourceBundle>(&mut self, system: System<C, R>) {
        self.systems.push(Box::new(system));
    }

    pub fn update(&mut self, world: &mut World) {
        self.run_all(world);
        self.flush_commands(world);
        self.sync(world);
    }

    pub fn run_all(&mut self, world: &mut World) {
        for system in self.systems.iter_mut() {
            system.run(&mut self.commands, world);
        }
    }

    pub fn flush_commands(&mut self, world: &mut World) {
        self.commands.flush(world);
    }

    pub fn sync(&mut self, world: &mut World) {
        for system in self.systems.iter_mut() {
            system.sync(world);
        }
    }
}
