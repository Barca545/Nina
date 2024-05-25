use self::{
  entities::{EntitiesInner, Entity},
  query::query::Query,
  resources::Resources
};
use crate::storage::{bundle::Bundle, type_info::TypeInfo, EcsData};
use eyre::Result;
use std::cell::{Ref, RefCell, RefMut};

pub mod command_buffer;
pub mod entities;
pub mod query;
pub mod resources;

//World must have mutation through &World
// Refactor:
// -Make a tests module
// -Make it so add_components panics if a component is unregistered
// -Move the big doctest example to the top of the module documentation
//  Update it to test for querying
// -Update `World` to `WorldInner` and have `World` be `Rc<WorldInner>`.
// -Do I need a reserve_entity method. Do I need a command buffer method since
// world is never mutably borrowed?

pub struct World {
  resources:Resources,
  entities:Entities
}

//Resource Implementation
impl World {
  ///Generates an empty [`World`].
  pub fn new() -> Self {
    World {
      resources:Default::default(),
      entities:Default::default()
    }
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
  /// Register type `T` as a component type.
  ///
  /// All types must be registered before they can be used as components.
  pub fn register_component<T:EcsData>(&self) -> &Self {
    self.entities.borrow_mut().register_component::<T>();
    self
  }

  /// Prepares the ECS for the insertion of data into a new `Entity`.
  ///
  /// The entity is initalized without any associated components.
  pub fn create_entity(&self) -> &Self {
    self.entities.borrow_mut().create_entity();
    self
  }

  /// Add a component of type `T` to the entity at `inserting_into_index`.
  ///
  /// Updates the entity's bitmap.
  ///
  /// # Panics
  ///
  /// Panics if `T` has not been registered.
  pub fn with_component<T:EcsData>(&self, data:T) -> Result<&Self> {
    self.entities.borrow_mut().with_component(data).unwrap();
    Ok(self)
  }

  /// Add a [`Bundle`] of components to the entity at `inserting_into_index`.
  ///
  /// Updates the entity's bitmap.
  ///
  /// # Panics
  ///
  /// Panics if `T` has not been registered.
  pub fn with_components(&self, bundle:impl Bundle) -> Result<()> {
    self.entities.borrow_mut().with_components(bundle)
  }

  ///Add a component to the entity.
  pub fn add_component<T:EcsData>(&self, entity:Entity, data:T) -> Result<()> {
    self.entities.borrow_mut().add_component(entity, data)
  }

  ///Add a [`Bundle`] of components to the entity.
  pub fn add_components(&self, entity:Entity, components:impl Bundle) -> Result<()> {
    self.entities.borrow_mut().add_components(entity, components)
  }

  ///Deletes an entity from the entities list matching the index.
  ///
  /// The next entity added will overwrite the emptied slot.
  pub fn delete_entity(&self, entity:Entity) -> Result<()> {
    self.entities.borrow_mut().delete_entity(entity)?;
    Ok(())
  }

  /// Delete a component from the entity.
  pub fn delete_component<T:EcsData>(&self, entity:Entity) -> Result<()> {
    self.entities.borrow_mut().delete_component::<T>(entity)
  }

  /// Delete a type-erased component from the entity.
  pub fn delete_component_erased(&self, entity:Entity, ty:TypeInfo) -> Result<()> {
    self.entities.borrow_mut().delete_component_erased(entity, ty)
  }
}

//Query implementation
impl World {
  pub fn query(&self) -> Query {
    Query::new(&self.entities)
  }
}

//CommandBuffer implementation
impl World {
  pub fn command_buffer(&self) {}
}

type Entities = RefCell<EntitiesInner>;

#[cfg(test)]
mod tests {
  //test for registering multiple components
}
