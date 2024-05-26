use self::{
  entities::{EntitiesInner, Entity},
  query::query::Query,
  resources::Resources
};
use crate::{
  errors::EcsErrors,
  storage::{Bundle, EcsData, TypeInfo}
};
use eyre::Result;

pub mod command_buffer;
pub mod entities;
pub mod query;
pub mod resources;

//World must have mutation through &World
// Refactor:
// -Make a tests module
// -Make it so add_components panics if a component is unregistered
// -Update `World` to `WorldInner` and have `World` be `Rc<WorldInner>`.
// -Do I need a reserve_entity method.
// -Do I need a command buffer method since
//  world is never mutably borrowed?
// -Steal the get components implementation from the query if speed becomes a
// concern

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
  pub fn add_resource(&mut self, data:impl EcsData) -> &mut Self {
    self.resources.add_resource(data);
    self
  }

  ///Query a resource by type and get a [`Ref<T>`].
  ///
  /// # Panics
  ///
  /// Panics if the resource has not been added.
  pub fn get_resource<T:EcsData>(&self) -> &T {
    self.resources.get::<T>()
  }

  ///Query a resource by type and get a mutable reference.
  ///
  /// # Panics
  ///
  /// Panics if the resource has not been added.
  pub fn get_resource_mut<T:EcsData>(&self) -> &mut T {
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
  pub fn register_component<T:EcsData>(&mut self) -> &mut Self {
    self.entities.register_component::<T>();
    self
  }

  /// Prepares the ECS for the insertion of data into a new `Entity`.
  ///
  /// The entity is initalized without any associated components.
  pub fn create_entity(&mut self) -> &mut Self {
    self.entities.create_entity();
    self
  }

  /// Reserves and returns a new `Entity`.
  pub fn reserve_entity(&mut self) -> Entity {
    self.entities.create_entity()
  }

  /// Add a component of type `T` to the entity at `inserting_into_index`.
  ///
  /// Updates the entity's bitmap.
  ///
  /// # Panics
  ///
  /// Panics if `T` has not been registered.
  pub fn with_component<T:EcsData>(&mut self, data:T) -> Result<&mut Self> {
    self.entities.with_component(data).unwrap();
    Ok(self)
  }

  /// Add a [`Bundle`] of components to the entity at `inserting_into_index`.
  ///
  /// Updates the entity's bitmap.
  ///
  /// # Panics
  ///
  /// Panics if `T` has not been registered.
  pub fn with_components<T:Bundle>(&mut self, bundle:T) -> Result<()> {
    self.entities.with_components(bundle)
  }

  /// Add a component to the entity.
  pub fn add_component<T:EcsData>(&mut self, entity:Entity, data:T) -> Result<()> {
    self.entities.add_component(entity, data)
  }

  /// Add a component to the entity.
  pub fn add_component_erased(&mut self, entity:Entity, ty:TypeInfo, ptr:*mut u8) -> Result<()> {
    self.entities.add_component_erased(entity, ty, ptr)
  }

  /// Add a [`Bundle`] of components to the entity.
  pub fn add_components<T:Bundle>(&mut self, entity:Entity, components:T) -> Result<()> {
    self.entities.add_components(entity, components)
  }

  /// Returns the component from the queried entity.
  ///
  /// # Panics
  ///
  /// Panics if the entity does not have the requested component.
  pub fn get_component<T:EcsData>(&self, entity:Entity) -> Result<&T> {
    let ty = TypeInfo::of::<T>();
    if self.entities.has_component::<T>(entity)? {
      return Ok(
        self
          .entities
          .components
          .get(&ty)
          .ok_or(EcsErrors::ComponentNotRegistered)?
          .get::<T>(entity)
      );
    } else {
      return Err(EcsErrors::ComponentDataDoesNotExist.into());
    }
  }

  /// Mutably returns the component from the queried entity.
  ///
  /// # Panics
  ///
  /// - Panics if the entity does not have the requested component.
  ///
  /// - Panics if the component is already borrowed in scope.
  pub fn get_component_mut<T:EcsData>(&self, entity:Entity) -> Result<&mut T> {
    let ty = TypeInfo::of::<T>();
    if self.entities.has_component::<T>(entity)? {
      return Ok(
        self
          .entities
          .components
          .get(&ty)
          .ok_or(EcsErrors::ComponentNotRegistered)?
          .get_mut::<T>(entity)
      );
    } else {
      return Err(EcsErrors::ComponentDataDoesNotExist.into());
    }
  }

  /// Deletes an entity from the entities list matching the index.
  ///
  /// The next entity added will overwrite the emptied slot.
  pub fn delete_entity(&mut self, entity:Entity) -> Result<()> {
    self.entities.delete_entity(entity)?;
    Ok(())
  }

  /// Delete a component from the entity.
  pub fn delete_component<T:EcsData>(&mut self, entity:Entity) -> Result<()> {
    self.entities.delete_component::<T>(entity)
  }

  /// Delete a type-erased component from the entity.
  pub fn delete_component_erased(&mut self, entity:Entity, ty:TypeInfo) -> Result<()> {
    self.entities.delete_component_erased(entity, ty)
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

type Entities = EntitiesInner;

#[cfg(test)]
mod tests {
  use super::World;

  #[test]
  fn systems_work() {
    let mut world = World::new();
    world.register_component::<Health>().register_component::<Armor>();
    world.add_resource(Resource(100));

    world.create_entity().with_components((Health(100.2), Armor(44))).unwrap();
    world.create_entity().with_component(Health(540.2)).unwrap();

    let p1_health = world.get_component::<Health>(0).unwrap();
    assert_eq!(p1_health.0, 100.2);

    let p1_health = world.get_component_mut::<Health>(1).unwrap();
    p1_health.0 = 100.0;
    assert_eq!(p1_health.0, 100.0);

    // some_system(&world);
  }

  fn some_system(world:&World) {
    let mut query = world.query();
    let entities = query.with_component::<Health>().unwrap().without_component::<Armor>().unwrap().run();

    // Check resources can be fetched and mutated
    let resource = world.get_resource_mut::<Resource>();
    resource.0 = 1002;

    // Check querying works
    for entity in entities {
      let health = entity.get_component::<Health>().unwrap();
      assert_eq!(health.0, 540.2)
    }

    let mut query = world.query();
    let entities = query.with_component::<Health>().unwrap().run();

    for entity in entities {
      let health = entity.get_component::<Health>().unwrap();
      dbg!(health);
      if let Ok(armor) = entity.get_component_mut::<Armor>() {
        assert_eq!(armor.0, 44);
        armor.0 += 6;
      }
    }

    let mut query = world.query();
    let entities = query.with_component::<Armor>().unwrap().run();

    for entity in entities {
      let armor = entity.get_component::<Armor>().unwrap();
      assert_eq!(armor.0, 50);
    }
  }

  #[derive(Debug)]
  struct Health(f32);
  struct Armor(u32);
  struct Resource(i32);
}
