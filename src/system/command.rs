use core::{
    alloc::Layout,
    marker::PhantomData,
    mem::{ManuallyDrop, MaybeUninit},
};

use crate::{component::Component, entity::Entity, World};

/// Stores commands to be executed on the world after the execution of all systems in a [Schedule]
///
/// # Implementation
/// Commands are stored in a contiguous vec of type-erased bytes. This is to avoid the overhead of
/// having a load of heap allocated trait objects, which would otherwise be necessary to store
/// commands of varying types in the same vec. Also, MaybeUninit is used to avoid wasted time
/// initialising the bytes, as they will be overwritten before being read.
///
/// As commands are of varying sizes, these leads to unaligned reads and writes, as the vec is packed.
/// It might be worth padding to the relevant alignment for each command type, when writing, to avoid
/// this, but I am unsure if this is worth it for now.
///
/// As commands are type-erased, the [CommandMetadata] for each command is stored in a separate vec, which
/// stores the memory layout of the command, and a function pointer to the command's execute function.
///
pub struct CommandQueue {
    commands: Vec<MaybeUninit<u8>>,

    // TODO: There are a fixed number of Commands, thus a fixed number of metadata, so could use an
    // array instead of a vec and index into it with the command's id?
    metadata: Vec<CommandMetadata>,
}

impl CommandQueue {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            metadata: Vec::new(),
        }
    }

    pub fn flag_modified<C: Component>(&mut self, entity: Entity) {
        self.push(FlagModifiedCommand::<C>::new(entity));
    }

    pub fn add_component<C: Component>(&mut self, entity: Entity, component: C) {
        self.push(AddComponentCommand::new(entity, component));
    }

    pub fn remove_component<C: Component>(&mut self, entity: Entity) {
        self.push(RemoveComponentCommand::<C>::new(entity));
    }

    pub fn add_entity(&mut self) {
        self.push(AddEntityCommand::new());
    }

    pub fn remove_entity(&mut self, entity: Entity) {
        self.push(RemoveEntityCommand::new(entity));
    }

    pub fn flush(&mut self, world: &mut World) {
        let mut ptr = self.commands.as_mut_ptr();

        for metadata in self.metadata.drain(..) {
            unsafe { (metadata.execute)(ptr, world) };
            unsafe { ptr = ptr.add(metadata.layout.size()) };
        }

        unsafe { self.commands.set_len(0) };
    }

    fn push<C: Command>(&mut self, command: C) {
        // `command` would be dropped in this scope, so ManuallyDrop here to avoid that as we are
        // manually moving it ourselves into the vec so will be responsible for dropping it later.
        // However, haven't actually implemented Drop for any commands yet, so this is not necessary

        let command = ManuallyDrop::new(command);
        let metadata = CommandMetadata::new::<C>();

        self.commands.reserve(metadata.layout.size());

        unsafe {
            let ptr = self.commands.as_mut_ptr().add(self.commands.len());
            ptr.cast::<ManuallyDrop<C>>().write_unaligned(command);

            self.commands
                .set_len(self.commands.len() + metadata.layout.size());
        }

        self.metadata.push(metadata);
    }
}

/// As commands are type erased, for the sake of contiguous storage, this stores necessary metadata
/// for a command to be executed on the world, such as memory layout and a function pointer to the
/// command's execute function
pub struct CommandMetadata {
    layout: Layout,
    execute: unsafe fn(command: *mut MaybeUninit<u8>, world: &mut World),
}

impl CommandMetadata {
    pub fn new<C: Command>() -> Self {
        Self {
            layout: Layout::new::<C>(),
            execute: |ptr, world| unsafe {
                let item = ptr.cast::<C>().read_unaligned();
                item.execute(world);
            },
        }
    }
}

pub trait Command {
    fn execute(self, world: &mut World);
}

pub struct AddComponentCommand<C: Component> {
    entity: Entity,
    component: C,
}

impl<C: Component> AddComponentCommand<C> {
    pub fn new(entity: Entity, component: C) -> Self {
        Self { entity, component }
    }
}

impl<C: Component> Command for AddComponentCommand<C> {
    fn execute(self, world: &mut World) {
        world.add_component(self.entity, self.component);
    }
}

pub struct RemoveComponentCommand<C: Component> {
    entity: Entity,
    _marker: PhantomData<C>,
}

impl<C: Component> RemoveComponentCommand<C> {
    pub fn new(entity: Entity) -> Self {
        Self {
            entity,
            _marker: core::marker::PhantomData,
        }
    }
}

impl<C: Component> Command for RemoveComponentCommand<C> {
    fn execute(self, world: &mut World) {
        world.remove_component::<C>(self.entity);
    }
}

pub struct AddEntityCommand {}

impl AddEntityCommand {
    pub fn new() -> Self {
        Self {}
    }
}

impl Command for AddEntityCommand {
    fn execute(self, world: &mut World) {
        world.create_entity();
    }
}

pub struct RemoveEntityCommand {
    entity: Entity,
}

impl RemoveEntityCommand {
    pub fn new(entity: Entity) -> Self {
        Self { entity }
    }
}

impl Command for RemoveEntityCommand {
    fn execute(self, world: &mut World) {
        world.delete_entity(self.entity);
    }
}

pub struct FlagModifiedCommand<C: Component> {
    entity: Entity,
    _marker: core::marker::PhantomData<C>,
}

impl<C: Component> FlagModifiedCommand<C> {
    pub fn new(entity: Entity) -> Self {
        Self {
            entity,
            _marker: core::marker::PhantomData,
        }
    }
}

impl<C: Component> Command for FlagModifiedCommand<C> {
    fn execute(self, world: &mut World) {
        let comp_id = world.component_manager.get_id::<C>();
        let entity_record = unsafe { world.entity_manager.get_record_unchecked(self.entity) };

        let archetype = unsafe { world.archetype_manager.get_mut_unchecked(&entity_record.archetype_id) };
        let storage = unsafe { archetype.get_mut_storage(comp_id) };

        debug_assert!(storage.is_tracked());

        let tracker = unsafe { storage.tracker.as_mut().unwrap_unchecked() };
        let info = unsafe { tracker.info.get_unchecked_mut(entity_record.archetype_row) };

        info.modified = world.tick;
        tracker.last_write = world.tick;
    }
}
