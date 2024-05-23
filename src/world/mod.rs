use self::{
  entities::{Entities, Entity},
  resources::Resources
};
use crate::storage::{bundle::Bundle, EcsData};
use std::cell::{Ref, RefCell, RefMut};

mod entities;
mod query;
mod resources;

//World must have mutation through &World
// Refactor:
// -Make a tests module
// -Make it so add_components panics if a component is unregistered

#[derive(Default)]
pub struct World {
  resources:Resources,
  entities:RefCell<Entities>
}

//Resource Implementation
impl World {
  ///Generates an empty [`World`].
  pub fn new() -> Self {
    Self::default()
  }

  ///Add a new resource to the world.
  pub fn add_resource(&self, data:impl EcsData) -> &Self {
    self.resources.add_resource(data);
    self
  }

  ///Query a resource by type and get a [`Ref<T>`].
  ///
  /// # Panics
  ///
  /// Panics if the resource has not been added.
  pub fn get_resource<T:EcsData>(&self) -> Ref<T> {
    self.resources.get::<T>()
  }

  ///Query a resource by type and get a mutable reference.
  ///
  /// # Panics
  ///
  /// Panics if the resource has not been added.
  pub fn get_resource_mut<T:EcsData>(&self) -> RefMut<T> {
    self.resources.get_mut::<T>()
  }

  ///Remove a resource from the [`World`].
  pub fn remove_resource<T:EcsData>(&mut self) {
    self.resources.remove::<T>()
  }
}

//Entity/Components Implementation
impl World {
  ///Updates the Entities to include components of type `T`.
  pub fn register_component<T:EcsData>(&self) -> &Self {
    self.entities.borrow_mut().register_component::<T>();
    self
  }

  /**
  Creates a new entity adds it to the entities list. Iterates over the
  registered components and initializes them with 'None'. Sets the bitmap
  for the entity to 0 indicating it has no components associated with it.

  # Example
  ```
  use nina::world::World;

  let world = World::new();
  world.create_entity()l
  ```
  */
  pub fn create_entity(&self) -> &Self {
    todo!()
  }

  /// Add a component of type `T` to the entity at `inserting_into_index`.
  ///
  /// Updates the entity's bitmap.
  ///
  /// # Panics
  ///
  /// Panics if `T` has not been registered.
  pub fn with_component() {}

  /// Add a [`Bundle`] of components to the entity at `inserting_into_index`.
  ///
  /// Updates the entity's bitmap.
  ///
  /// # Panics
  ///
  /// Panics if `T` has not been registered.
  pub fn with_components() {}

  ///Add a component to the provided entity.
  pub fn add_component(entity:Entity) {}

  ///Add a [`Bundle`] of components to the provided entity.
  ///
  /// # Warning
  ///
  /// Does not error if component is unregistered but operation will fail.
  pub fn add_components(entity:Entity, components:impl Bundle) {}

  ///Deletes an entity from the entities list matching the index.
  /// Leaves the slot open -- the next entity added will overwrite the emptied
  /// slot.
  pub fn delete_entity() {}
}

#[cfg(test)]
mod tests {
  //test for registering multiple components
}
