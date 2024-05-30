use crate::{
  errors::EcsErrors,
  storage::{EcsData, TypeInfo},
  world::Entities
};
use eyre::Result;

/// Structure which references an entity located by a
/// [`Query`](super::query::Query).
pub struct QueryEntity<'a> {
  pub id:usize,
  entities:&'a Entities
}

impl<'a> QueryEntity<'a> {
  pub fn new(id:usize, entities:&'a Entities) -> Self {
    Self { id, entities }
  }

  /// Fetches a component of type `T` from a queried entity.
  ///
  /// # Panics
  /// - Panics if the entity does not have the component.
  pub fn get_component<T:EcsData>(&self) -> Result<&T> {
    let ty = TypeInfo::of::<T>();
    let entities = self.entities;

    if entities.has_component::<T>(self.id)? {
      let components = entities.components.get(&ty).unwrap();
      // This is essentially the same as `ErasedVec`'s get method but skips the checks
      // because they are redundant
      return Ok(unsafe { &*components.indexed_ptr::<T>(self.id) });
    } else {
      return Err(
        EcsErrors::ComponentDataDoesNotExist {
          entity:self.id,
          ty:ty.name()
        }
        .into()
      );
    }
  }

  /// Mutably fetches a component of type `T` from a queried entity.
  ///
  /// # Panics
  /// - Panics if the entity does not have the component.
  pub fn get_component_mut<T:EcsData>(&self) -> Result<&mut T> {
    let ty = TypeInfo::of::<T>();
    let entities = self.entities;

    if entities.has_component::<T>(self.id)? {
      let components = entities.components.get(&ty).unwrap();
      // This is essentially the same as `ErasedVec`'s get method but skips the checks
      // because they are redundant
      return Ok(unsafe { &mut *components.indexed_ptr::<T>(self.id) });
    } else {
      return Err(
        EcsErrors::ComponentDataDoesNotExist {
          entity:self.id,
          ty:ty.name()
        }
        .into()
      );
    }
  }
}
