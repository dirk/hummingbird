/// Inkwell's representation of modules has a nasty lifetime bound on the
/// context that makes it impossible to store them for the duration of
/// compilation. Therefore we have to cheat the borrow-checker.
use std::fmt::{Debug, Error, Formatter};
use std::intrinsics::transmute;
use std::ops::Deref;

use inkwell::builder::Builder;
use inkwell::module::Module;
use inkwell::types::FunctionType;
use inkwell::values::{FunctionValue, IntValue};

macro_rules! opaque {
    ($typ:ident, $size:literal) => {
        // Assume `$typ` is `Module` in the examples below.
        paste::item! {
            type [<$typ Size>] = [usize; $size];

            // Generate an `OpaqueModule` holding a `ModuleSize`.
            #[derive(Clone)]
            pub struct [<Opaque $typ>]([<$typ Size>]);

            impl [<Opaque $typ>] {
                pub fn wrap<'ctx>(clear: $typ<'ctx>) -> Self {
                    let opaque = unsafe { transmute::<$typ<'ctx>, [<$typ Size>]>(clear) };
                    Self(opaque)
                }

                pub fn unwrap<'a, 'ctx>(&'a self) -> &'a $typ<'ctx> {
                    unsafe { transmute::<&[<$typ Size>], &$typ<'ctx>>(&self.0) }
                }
            }

            impl Debug for [<Opaque $typ>] {
                fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
                    write!(f, "Opaque{}({:?}", stringify!($typ), self.unwrap())
                }
            }
        }
    };
}

opaque!(Builder, 1);
opaque!(FunctionValue, 1);
opaque!(FunctionType, 1);
opaque!(IntValue, 1);
opaque!(Module, 11);
