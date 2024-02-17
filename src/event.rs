use crate::{resource::ResourceManager, ResourceId};

pub struct EventManager {
    event_lists: Vec<usize>,
}

impl EventManager {
    pub fn new() -> Self {
        Self {
            event_lists: Vec::new(),
        }
    }

    pub fn register_event<T: 'static>(&mut self, id: ResourceId<T>) {
        self.event_lists.push(id.index);
    }

    pub fn clear_events(&self, resource_manager: &ResourceManager) {
        // for event_list in &self.event_lists {
        //     let events = unsafe { resource_manager.get_mut_unchecked(*event_list) };
        // }

        todo!()
    }
}

pub struct Events<T> {
    read: Vec<T>,
    write: Vec<T>,
}

impl<T> Events<T> {
    pub fn new() -> Self {
        Self {
            read: Vec::new(),
            write: Vec::new(),
        }
    }

    pub fn push(&mut self, event: T) {
        self.write.push(event);
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.read.iter()
    }

    pub fn clear(&mut self) {
        self.read.clear();
        std::mem::swap(&mut self.read, &mut self.write);
    }
}
