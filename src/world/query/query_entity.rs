use crate::{
  errors::EcsErrors,
  storage::{EcsData, TypeInfo},
  world::Entities
};
use eyre::Result;

/// Structure which references an entity located by a `Query`.
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
  ///
  /// Panics if the entity does not have the component.
  pub fn get_component<T:EcsData>(&self) -> Result<&T> {
    let ty = TypeInfo::of::<T>();
    let entities = self.entities.borrow();

    if entities.has_component::<T>(self.id)? {
      let components = entities.components.get(&ty).unwrap();
      // This is essentially the same as `ErasedVec`'s get method but skips the checks
      // because they are redundant
      return Ok(unsafe { &*components.indexed_ptr::<T>(self.id) });
    } else {
      return Err(EcsErrors::ComponentDataDoesNotExist.into());
    }
  }

  /// Mutably fetches a component of type `T` from a queried entity.
  ///
  /// # Panics
  ///
  /// Panics if the entity does not have the component.
  pub fn get_component_mut<T:EcsData>(&self) -> Result<&mut T> {
    let ty = TypeInfo::of::<T>();
    let entities = self.entities.borrow();

    if entities.has_component::<T>(self.id)? {
      let components = entities.components.get(&ty).unwrap();
      // This is essentially the same as `ErasedVec`'s get method but skips the checks
      // because they are redundant
      return Ok(unsafe { &mut *components.indexed_ptr::<T>(self.id) });
    } else {
      return Err(EcsErrors::ComponentDataDoesNotExist.into());
    }
  }

  /// Add a component to the entity referenced by the [`QueryEntity`].
  pub fn add_component<T:EcsData>(&self, component:T) -> Result<()> {
    self.entities.borrow_mut().add_component(self.id, component)
  }

  // ///Returns an `Rc` smart pointer to the component.
  // pub fn get_commonent_ref<T:Any>(&self) -> Result<ComponentRef<T>> {
  //   let components = self.extract_components::<T>()?;
  //   let component = components[self.id].as_ref();

  //   match component {
  //     Some(component) => {
  //       let component = component;
  //       let component_ref = ComponentRef::new::<T>(component.clone());
  //       Ok(component_ref)
  //     }
  //     None => Err(EcsErrors::ComponentDataDoesNotExist.into())
  //   }
  // }
}
