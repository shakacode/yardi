#![feature(proc_macro_hygiene)]

pub extern crate utils;

pub use dependencies_macro::dependencies;
pub use lazy_static;
pub use utils::Inject;

#[macro_export]
macro_rules! inject {
    ($inj: expr, $dep: path) => {
        <Injector as Inject<$dep>>::inject(&$inj)
    }
}
