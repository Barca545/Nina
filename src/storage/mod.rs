use std::any::Any;

pub mod bundle;
pub mod erased_collections;
pub mod type_info;
pub mod type_map;

/// Types that can be components.
///
/// This is just a convenient shorthand for `Send + Sync + 'static`, and never
/// needs to be implemented manually.
pub trait EcsData: 'static + Any + Send + Sync {}
impl<T:Send + Sync + 'static> EcsData for T {}
