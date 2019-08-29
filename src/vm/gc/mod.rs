mod gc_allocator;
mod gc_ptr;

pub use gc_allocator::GcAllocator;
pub use gc_ptr::GcPtr;

// NOTE: These are separate traits so that we don't conflate being managed by
// GC with being traceable. For example, stack frames are not GC-managed but
// they do play a big part in tracing.

/// Types which can be managed by the GC (ie. allocated and deallocated by it)
/// must have this trait.
pub trait GcManaged {}

/// Types which contain `GcPtr`s or which contain types which contain `GcPtr`s
/// (and so on) must implement this trait.
///
/// Tracing is visiting the "nodes" of the memory tree, whereas `GcPtr::mark`
/// is for the "leaves" of the tree. For example, for a `Value` that is a map
/// with a string key and array-of-string value:
///
/// - Value::trace(map)
///   - GcPtr<Map>::mark()
///   - Map::trace()
///     - for (key, value) in self
///       - Value::trace(key)
///         - GcPtr<String>::mark()
///       - Value::trace(value)
///         - GcPtr<Array>::mark()
///         - Array::trace()
///           - for item in self
///             - Value::trace(item)
///               - GcPtr<String>::mark()
///
/// This is slightly more verbose than letting `GcTrace::trace` just do
/// everything, but it has the advantage of static (and enum) dispatch which
/// is much faster than dynamic dispatch.
pub trait GcTrace {
    /// Mark any `GcPtr`s in self or self's children.
    fn trace(&self);
}
