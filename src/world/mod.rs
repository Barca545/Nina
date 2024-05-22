use self::{entities::Entities, resources::Resources};
use crate::storage::EcsData;
use std::cell::{Ref, RefMut};

mod entities;
mod resources;

//World must have mutation through &World
// Refactor:
// -Make a tests module

#[derive(Default)]
pub struct World {
  resources:Resources,
  entities:Entities
}

impl World {
  ///Generates an empty [`World`].
  pub fn new() -> Self {
    Self::default()
  }

  ///Add a new resource to the world.
  pub fn add_resource(&mut self, data:impl EcsData) -> &mut Self {
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

#[cfg(test)]
mod tests {}
