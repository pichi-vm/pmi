#![allow(missing_docs)]

pub mod aarch64;
pub mod x86_64;

fn is_zero<T: Default + PartialEq>(v: &T) -> bool {
    v == &T::default()
}
