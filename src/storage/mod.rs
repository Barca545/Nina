use std::any::Any;

mod bundle;
mod erased_collections;
mod type_info;
mod type_map;

pub use self::{bundle::*, erased_collections::*, type_info::*, type_map::*};

/// Types that can be components.
///
/// This is just a convenient shorthand for `'static + Any`, and never
/// needs to be implemented manually.
pub trait EcsData: 'static + Any {}
impl<T:'static> EcsData for T {}
