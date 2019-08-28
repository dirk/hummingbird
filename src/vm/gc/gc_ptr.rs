use std::ops::{Deref, DerefMut};

use super::GcTrace;

/// Heap-allocated box holding a GC'ed value.
pub struct GcBox<T: GcTrace + ?Sized> {
    marked: bool,
    value: T,
}

impl<T: GcTrace> GcBox<T> {
    pub fn new(value: T) -> Self {
        Self {
            marked: false,
            value,
        }
    }
}

impl<T: GcTrace + ?Sized> GcBox<T> {
    pub fn is_marked(&self) -> bool {
        self.marked
    }

    pub fn mark(&mut self) {
        self.marked = true;
    }

    pub fn unmark(&mut self) {
        self.marked = false;
    }
}

/// Pointer to a box with dynamically-checked mutability.
#[derive(Debug)]
pub struct GcPtr<T: GcTrace> {
    boxed: *mut GcBox<T>,
}

impl<T: GcTrace> GcPtr<T> {
    pub fn new(boxed: *mut GcBox<T>) -> Self {
        Self { boxed }
    }

    fn mark(&self) {
        unsafe {
            (*self.boxed).mark();
        }
    }
}

impl<T: GcTrace> Clone for GcPtr<T> {
    fn clone(&self) -> Self {
        Self { boxed: self.boxed }
    }
}

impl<T: GcTrace> Deref for GcPtr<T> {
    type Target = T;

    fn deref(&self) -> &T {
        let value = unsafe { &(*self.boxed).value };
        value
    }
}

impl<T: GcTrace> DerefMut for GcPtr<T> {
    fn deref_mut(&mut self) -> &mut T {
        let value = unsafe { &mut (*self.boxed).value };
        value
    }
}

impl<T: GcTrace> GcTrace for GcPtr<T> {
    /// Tracing a `GcPtr` will mark it and then trace its contents.
    fn trace(&self) {
        self.mark();
        let value = &**self;
        value.trace();
    }
}
