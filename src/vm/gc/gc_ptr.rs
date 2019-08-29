use std::ops::{Deref, DerefMut};

use super::{GcManaged, GcTrace};

/// Heap-allocated box holding a GC'ed value.
pub struct GcBox<T: GcManaged + ?Sized> {
    marked: bool,
    value: T,
}

impl<T: GcManaged> GcBox<T> {
    pub fn new(value: T) -> Self {
        Self {
            marked: false,
            value,
        }
    }
}

impl<T: GcManaged + ?Sized> GcBox<T> {
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
pub struct GcPtr<T: GcManaged> {
    boxed: *mut GcBox<T>,
}

impl<T: GcManaged> GcPtr<T> {
    pub fn new(boxed: *mut GcBox<T>) -> Self {
        Self { boxed }
    }

    pub fn mark(&self) {
        unsafe {
            (*self.boxed).mark();
        }
    }
}

impl<T: GcManaged> Clone for GcPtr<T> {
    fn clone(&self) -> Self {
        Self { boxed: self.boxed }
    }
}

impl<T: GcManaged> Deref for GcPtr<T> {
    type Target = T;

    fn deref(&self) -> &T {
        let value = unsafe { &(*self.boxed).value };
        value
    }
}

impl<T: GcManaged> DerefMut for GcPtr<T> {
    fn deref_mut(&mut self) -> &mut T {
        let value = unsafe { &mut (*self.boxed).value };
        value
    }
}

/// For types which support `GcTrace` add an automatic convenience
/// implementation to mark the pointer and then trace its contents.
impl<T: GcManaged + GcTrace> GcTrace for GcPtr<T> {
    /// Tracing a `GcPtr` will mark it and then trace its contents.
    fn trace(&self) {
        self.mark();
        let value = &**self;
        value.trace();
    }
}
