use crate::resource::ResourceManager;

pub struct EventManager {
    event_lists: Vec<EventMetadata>,
}

impl EventManager {
    pub fn new() -> Self {
        Self {
            event_lists: Vec::new(),
        }
    }

    pub fn register_event<E: 'static>(&mut self, resource_manager: &mut ResourceManager) {
        let index = resource_manager.len();

        let events = Events::<E>::new();
        let metadata = EventMetadata {
            index,
            clear: |ptr| unsafe {
                let events = &mut *(ptr as *mut Events<E>);
                events.write.clear();
            },
        };

        resource_manager.add(events);
        self.event_lists.push(metadata);
    }

    pub fn clear_events(&self, resource_manager: &mut ResourceManager) {
        for event_list in &self.event_lists {
            unsafe {
                (event_list.clear)(resource_manager.get_ptr(event_list.index));
            }
        }
    }
}

pub struct Events<E> {
    read: Vec<E>,
    write: Vec<E>,
}

impl<E> Events<E> {
    pub fn new() -> Self {
        Self {
            read: Vec::new(),
            write: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.read.len()
    }

    pub fn push(&mut self, event: E) {
        self.write.push(event);
    }

    pub fn iter(&self) -> impl Iterator<Item = &E> {
        self.read.iter()
    }

    pub fn clear(&mut self) {
        self.read.clear();
        core::mem::swap(&mut self.read, &mut self.write);
    }
}

pub struct EventMetadata {
    pub index: usize,
    pub clear: unsafe fn(*mut u8),
}
