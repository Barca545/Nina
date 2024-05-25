use crate::{
  errors::EcsErrors,
  storage::{Bundle, EcsData, ErasedVec, TypeInfo, TypeMap}
};
use eyre::Result;

// Refactor:
// -Implement tests for inserting and deleting erased
// -Add add_components_erased, delete_components_erased, and with_components

pub type Entity = usize;

#[derive(Default)]
pub struct EntitiesInner {
  pub components:TypeMap<ErasedVec>,
  /// Contains the bitmasks for registered components.
  bitmasks:TypeMap<u128>,
  /// Vector of entity bitmasks.
  pub map:Vec<u128>,
  inserting_into_index:Entity
}

impl EntitiesInner {
  /// Register type `T` as a component type.
  ///
  /// All types must be registered before they can be used as components.
  pub fn register_component<T:EcsData>(&mut self) {
    let ty = TypeInfo::of::<T>();
    // Create new component storage
    self.components.insert(ty, ErasedVec::new::<T>());

    // Create a new bitmask for the type
    self.bitmasks.insert(ty, 1 << self.bitmasks.len());
  }

  /// Returns the next free entity id for insertion.
  pub fn create_entity(&mut self) {
    if let Some((index, _)) = self.map.iter().enumerate().find(|(_index, mask)| **mask == 0) {
      self.inserting_into_index = index;
    }
    // If there are no free entity slots grow the entities struct
    else {
      self.components.iter_mut().for_each(|(_key, components)| components.pad());
      self.map.push(0);
      self.inserting_into_index = self.map.len() - 1;
    }
  }

  /// Add a component of type `T` to the entity at `inserting_into_index`.
  ///
  /// Updates the entity's bitmap.
  ///
  /// # Panics
  ///
  /// Panics if `T` has not been registered.
  pub fn with_component<T:EcsData>(&mut self, data:T) -> Result<()> {
    let ty = TypeInfo::of::<T>();
    let index = self.inserting_into_index;

    if let Some(components) = self.components.get_mut(&ty) {
      let component = components.get_mut::<T>(index);
      *component = data;

      let bitmask = self.bitmasks.get(&ty).unwrap();
      self.map[index] |= *bitmask
    }
    // Return an error if the component type was never registered
    else {
      return Err(EcsErrors::CreateComponentNeverCalled { component:ty.name() }.into());
    };
    Ok(())
  }

  /// Add a [`Bundle`] of components to the entity at `inserting_into_index`.
  ///
  /// Updates the entity's bitmap.
  ///
  /// # Panics
  ///
  /// Panics if `T` has not been registered.
  pub fn with_components<T:Bundle>(&mut self, bundle:T) -> Result<()> {
    unsafe {
      bundle.put(|ptr, ty| {
        let entity = self.inserting_into_index;

        if let Some(components) = self.components.get_mut(&ty) {
          components.insert_erased(ptr, ty, entity);

          let bitmask = self.bitmasks.get(&ty).unwrap();
          self.map[entity] |= *bitmask;
          Ok(())
        } else {
          return Err(EcsErrors::CreateComponentNeverCalled { component:ty.name() }.into());
        }
      })
    }
  }

  /// Delete a component from the entity.
  pub fn delete_component<T:EcsData>(&mut self, entity:Entity) -> Result<()> {
    let ty = TypeInfo::of::<T>();
    if let Some(mask) = self.bitmasks.get(&ty) {
      self.map[entity] &= !*mask;
    }
    Ok(())
  }

  /// Delete a type-erased component from the entity.
  pub fn delete_component_erased(&mut self, entity:Entity, ty:TypeInfo) -> Result<()> {
    if let Some(mask) = self.bitmasks.get(&ty) {
      self.map[entity] &= !*mask;
    }
    Ok(())
  }

  /// Add a component to the provided entity.
  ///
  /// Updates the entity's bitmap.
  ///
  /// # Panics
  ///
  /// Panics if `T` has not been registered.
  pub fn add_component<T:EcsData>(&mut self, entity:Entity, component:T) -> Result<()> {
    let ty = TypeInfo::of::<T>();

    if let Some(mask) = self.bitmasks.get(&ty) {
      self.map[entity] |= *mask;
    } else {
      return Err(EcsErrors::ComponentNotRegistered.into());
    };

    let component_slot = self.components.get_mut(&ty).unwrap().get_mut::<T>(entity);
    *component_slot = component;

    Ok(())
  }

  /// Add a type-erased component to the entity.
  ///
  /// Updates the entity's bitmap.
  ///
  /// # Panics
  ///
  /// Panics if `T` has not been registered.
  pub fn add_component_erased<T:EcsData>(&mut self, entity:Entity, data:T) -> Result<()> {
    self.add_components(entity, (data,))
  }

  /// Add a [`Bundle`] of components to the provided entity.
  ///
  /// # Panics
  ///
  /// Panics if a component's type has not been registered.
  pub fn add_components<T:Bundle>(&mut self, entity:Entity, components:T) -> Result<()> {
    unsafe {
      components.put(|ptr, ty| {
        if let Some(components) = self.components.get_mut(&ty) {
          components.insert_erased(ptr, ty, entity);

          let bitmask = self.bitmasks.get(&ty).unwrap();
          self.map[entity] |= *bitmask;
          Ok(())
        } else {
          return Err(EcsErrors::CreateComponentNeverCalled { component:ty.name() }.into());
        }
      })
    }
  }

  /// Returns the component from the queried entity.
  ///
  /// # Panics
  ///
  /// Panics if the entity does not have the requested component.
  pub fn get_component<T:EcsData>(&self, entity:Entity) -> Result<&T> {
    let ty = TypeInfo::of::<T>();
    if self.has_component::<T>(entity).unwrap() {
      return Ok(self.components.get(&ty).unwrap().get::<T>(entity));
    } else {
      return Err(EcsErrors::ComponentDataDoesNotExist.into());
    }
  }

  /// Mutably returns the component from the queried entity.
  ///
  /// # Panics
  ///
  /// Panics if the entity does not have the requested component.
  pub fn get_component_mut<T:EcsData>(&self, entity:Entity) -> Result<&mut T> {
    let ty = TypeInfo::of::<T>();
    if self.has_component::<T>(entity).unwrap() {
      return Ok(self.components.get(&ty).unwrap().get_mut::<T>(entity));
    } else {
      return Err(EcsErrors::ComponentDataDoesNotExist.into());
    }
  }

  /// Deletes an entity from the entities list matching the index.
  ///
  /// The next entity added will overwrite the emptied slot.
  pub fn delete_entity(&mut self, entity:Entity) -> Result<()> {
    if let Some(map) = self.map.get_mut(entity) {
      *map = 0;
    } else {
      return Err(EcsErrors::EntityDoesNotExist.into());
    }

    Ok(())
  }

  ///Returns an [`Option<u128>`] containing the `bitmask`of a given
  /// [`TypeInfo`].
  pub fn get_bitmask(&self, ty:&TypeInfo) -> Option<u128> {
    self.bitmasks.get(ty).copied()
  }

  ///Checks whether an entity has a component of type `T` and returns a
  /// [`Result<bool>`].
  ///
  /// # Panics
  ///
  /// Panics if the component was never registered;
  pub fn has_component<T:EcsData>(&self, entity:Entity) -> Result<bool> {
    let ty = TypeInfo::of::<T>();

    match self.get_bitmask(&ty) {
      Some(mask) => Ok((self.map[entity] & mask) != 0),
      None => Err(EcsErrors::ComponentNotRegistered.into())
    }
  }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
  use super::*;

  #[test]
  fn register_an_entity() {
    let mut entities:EntitiesInner = EntitiesInner::default();
    let ty = TypeInfo::of::<Health>();
    entities.register_component::<Health>();
    let health_components = entities.components.get(&ty).unwrap();
    assert_eq!(health_components.len(), 0);
  }

  #[test]
  fn bitmask_updated_when_register_a_component() {
    let mut entities:EntitiesInner = EntitiesInner::default();

    entities.register_component::<Health>();
    let typeid = TypeInfo::of::<Health>();
    let mask = entities.bitmasks.get(&typeid).unwrap();
    assert_eq!(*mask, 1);

    entities.register_component::<Speed>();
    let typeid = TypeInfo::of::<Speed>();
    let mask = entities.bitmasks.get(&typeid).unwrap();
    assert_eq!(*mask, 2);
  }

  #[test]
  fn create_an_entity() {
    let mut entities:EntitiesInner = EntitiesInner::default();
    entities.register_component::<Health>();
    entities.register_component::<Speed>();
    entities.create_entity();
    let healths = entities.components.get(&TypeInfo::of::<Health>()).unwrap();
    let speeds = entities.components.get(&TypeInfo::of::<Speed>()).unwrap();

    //Confirm the entity's slot is padded
    assert!(healths.len() == speeds.len() && healths.len() == 1);

    let health_data = unsafe { healths.get_unchecked::<[u8; 4]>(0) };
    assert_eq!(health_data, &[0; 4]);

    let speed_data = unsafe { speeds.get_unchecked::<[u8; 4]>(0) };
    assert_eq!(speed_data, &[0; 4]);
  }

  #[test]
  fn create_with_component() -> Result<()> {
    let mut entities:EntitiesInner = EntitiesInner::default();
    entities.register_component::<Health>();
    entities.register_component::<Speed>();

    entities.create_entity();
    entities.with_component(Health(100))?;
    entities.with_component(Speed(15))?;

    let health = entities.components.get(&TypeInfo::of::<Health>()).unwrap().get::<Health>(0);
    assert_eq!(health.0, 100);
    let speed = entities.components.get(&TypeInfo::of::<Speed>()).unwrap().get::<Speed>(0);
    assert_eq!(speed.0, 15);
    Ok(())
  }

  #[test]
  fn create_with_component_bundle() -> Result<()> {
    let mut entities:EntitiesInner = EntitiesInner::default();
    entities.register_component::<Health>();
    entities.register_component::<Speed>();

    entities.create_entity();

    entities.with_components((Health(100), Speed(15)))?;

    let health = entities.components.get(&TypeInfo::of::<Health>()).unwrap().get::<Health>(0);
    assert_eq!(health.0, 100);
    let speed = entities.components.get(&TypeInfo::of::<Speed>()).unwrap().get::<Speed>(0);
    assert_eq!(speed.0, 15);

    Ok(())
  }

  #[test]
  fn map_is_updated_when_creating_entities() -> Result<()> {
    let mut entities:EntitiesInner = EntitiesInner::default();

    entities.register_component::<Health>();
    entities.register_component::<Speed>();

    entities.create_entity();
    entities.with_component(Health(100))?;
    entities.with_component(Speed(15))?;

    let entity_map = entities.map[0];
    assert_eq!(entity_map, 3);

    entities.create_entity();
    entities.with_component(Speed(15))?;

    let entity_map = entities.map[1];
    assert_eq!(entity_map, 2);

    Ok(())
  }

  #[test]
  fn delete_component_by_entity_id() -> Result<()> {
    let mut entities = EntitiesInner::default();

    entities.register_component::<Health>();
    entities.register_component::<Speed>();
    entities.register_component::<Damage>();

    entities.create_entity();
    entities.with_component(Health(100))?;
    entities.with_component(Speed(50))?;
    entities.with_component(Damage(50))?;

    assert_eq!(entities.map[0], 7);

    entities.delete_component::<Health>(0)?;

    assert_eq!(entities.map[0], 6);

    Ok(())
  }

  #[test]
  fn delete_component_by_entity_id_erased() -> Result<()> {
    let mut entities = EntitiesInner::default();

    entities.register_component::<Health>();
    entities.register_component::<Speed>();
    entities.register_component::<Damage>();

    entities.create_entity();
    entities.with_component(Health(100))?;
    entities.with_component(Speed(50))?;
    entities.with_component(Damage(50))?;

    assert_eq!(entities.map[0], 7);

    entities.delete_component_erased(0, TypeInfo::of::<Health>())?;

    assert_eq!(entities.map[0], 6);

    Ok(())
  }

  #[test]
  fn add_component_to_entity_by_id() -> Result<()> {
    let mut entities = EntitiesInner::default();

    entities.register_component::<Health>();
    entities.register_component::<Speed>();

    entities.create_entity();
    entities.with_component(Health(100))?;

    //how are we finding the entity's id?
    entities.add_component(0, Speed(50))?;

    assert_eq!(entities.map[0], 3);

    let speed_ty = TypeInfo::of::<Speed>();
    let speed = entities.components.get(&speed_ty).unwrap().get::<Speed>(0);

    assert_eq!(speed.0, 50);

    Ok(())
  }

  #[test]
  fn add_component_to_entity_by_id_erased() -> Result<()> {
    let mut entities = EntitiesInner::default();

    entities.register_component::<Health>();
    entities.register_component::<Speed>();

    entities.create_entity();
    entities.with_component(Health(100))?;
    entities.add_component_erased(0, Speed(50))?;

    entities.create_entity();
    entities.add_component_erased(1, Speed(131))?;

    assert_eq!(entities.map[0], 3);

    let speed_ty = TypeInfo::of::<Speed>();
    let speed_1 = entities.components.get(&speed_ty).unwrap().get::<Speed>(0);

    assert_eq!(speed_1.0, 50);

    let speed_2 = entities.components.get(&speed_ty).unwrap().get::<Speed>(1);

    assert_eq!(speed_2.0, 131);

    Ok(())
  }

  #[test]
  fn delete_entity_by_id() -> Result<()> {
    let mut entities = EntitiesInner::default();

    entities.register_component::<Health>();

    entities.create_entity();
    entities.with_component(Health(100))?;

    entities.delete_entity(0)?;

    assert_eq!(entities.map[0], 0);

    Ok(())
  }

  #[test]
  fn created_entities_are_inserted_into_deleted_entities_columns() -> Result<()> {
    let mut entities = EntitiesInner::default();
    entities.register_component::<Health>();
    entities.register_component::<Speed>();

    entities.create_entity();
    entities.with_component(Health(100))?;

    entities.create_entity();
    entities.with_component(Health(50))?;

    entities.delete_entity(0)?;

    entities.create_entity();
    entities.with_component(Health(25))?;

    assert_eq!(entities.map[0], 1);

    let ty = TypeInfo::of::<Health>();
    let health = entities.components.get(&ty).unwrap().get::<Health>(0);

    assert_eq!(health.0, 25);

    Ok(())
  }

  struct Health(pub u32);
  struct Speed(pub u32);
  struct Damage(pub u32);
}
