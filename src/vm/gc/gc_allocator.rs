// GcBox is a private implementation detail of the GC.
use super::gc_ptr::GcBox;
use super::GcPtr;

/// Manages allocation and collection of objects for a stack.
pub struct GcAllocator {
    slots: Vec<*mut GcBox<dyn GcTrace>>,
    debug: bool,
}

impl GcAllocator {
    pub fn new(debug: bool) -> Self {
        Self {
            slots: vec![],
            debug,
        }
    }

    pub fn allocate<T: GcTrace + 'static>(&mut self, value: T) -> GcPtr<T> {
        let boxed = Box::into_raw(Box::new(GcBox::new(value)));
        self.slots.push(boxed);
        GcPtr::new(boxed)
    }

    /// Right now we'll tell the VM we could always run the GC.
    /// FIXME: Build a better heuristic for running GC.
    pub fn needs_collection(&self) -> bool {
        true
    }

    pub fn collect<F: FnOnce() -> ()>(&mut self, tracer: F) {
        self.print_debug(format!("start slots={:?}", self.slots.len()));
        self.unmark();

        // This should trace all the objects. Any not traced will be dropped!
        tracer();

        let mut index = 0;
        while index != self.slots.len() {
            let slot = self.slots[index];
            let marked = unsafe { (*slot).is_marked() };
            self.print_debug(format!("visit {:?} marked={:?}", slot, marked));
            if !marked {
                let removed = self.slots.remove(index);
                // Convert it back into a box to be dropped.
                unsafe { Box::from_raw(removed) };
            } else {
                index += 1;
            }
        }
        self.print_debug(format!("finish slots={:?}", self.slots.len()));
    }

    fn unmark(&mut self) {
        for slot in self.slots.iter_mut() {
            unsafe {
                (**slot).unmark();
            }
        }
    }

    fn print_debug<S: Into<String>>(&self, message: S) {
        if self.debug {
            println!("GC: {}", message.into())
        }
    }
}

pub trait GcTrace {
    /// Mark any `GcPtr`s in self or self's children.
    fn trace(&self) -> ();
}
