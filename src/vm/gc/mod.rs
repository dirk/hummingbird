mod gc_allocator;
mod gc_ptr;

pub use gc_allocator::GcAllocator;
pub use gc_ptr::GcPtr;

/// Types which can be managed by the GC (ie. allocated and deallocated by it)
/// must have this trait.
pub trait GcManaged {}

/// Types which contain `GcPtr`s or which contain types which contain `GcPtr`s
/// (and so on) must implement this trait.
pub trait GcTrace {
    /// Mark any `GcPtr`s in self or self's children.
    fn trace(&self);
}
