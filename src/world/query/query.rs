use super::query_entity::QueryEntity;
use crate::{
  errors::EcsErrors,
  storage::{type_info::TypeInfo, EcsData},
  world::Entities
};
use eyre::Result;

// #[derive(Debug)]
pub struct Query<'a> {
  map:u128,
  exclude_map:u128,
  entities:&'a Entities
}

impl<'a> Query<'a> {
  ///Create a new [`Query`].
  pub fn new(entities:&'a Entities) -> Self {
    Self {
      map:0,
      exclude_map:0,
      entities
    }
  }

  ///Register a component the queried entities must hold.
  pub fn with_component<T:EcsData>(&mut self) -> Result<&mut Self> {
    let ty = TypeInfo::of::<T>();
    if let Some(bit_mask) = self.entities.borrow().get_bitmask(&ty) {
      self.map |= bit_mask;
    } else {
      return Err(EcsErrors::ComponentNotRegistered.into());
    }
    Ok(self)
  }

  ///Register a component the queried entities must not hold.
  pub fn without_component<T:EcsData>(&mut self) -> Result<&mut Self> {
    let ty = TypeInfo::of::<T>();
    if let Some(bit_mask) = self.entities.borrow().get_bitmask(&ty) {
      self.exclude_map |= bit_mask;
    } else {
      return Err(EcsErrors::ComponentNotRegistered.into());
    }
    Ok(self)
  }

  ///Consumes the [`Query`]. Returns a [`Vec`] of [`QueryEntity`] containing
  /// all entities who hold the queried components.
  pub fn run(&self) -> Vec<QueryEntity> {
    self
      .entities
      .borrow()
      .map
      .iter()
      .enumerate()
      .filter_map(|(index, entity_map)| {
        if (entity_map & (self.map | self.exclude_map)) == self.map {
          Some(QueryEntity::new(index, self.entities))
        } else {
          None
        }
      })
      .collect()
  }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod test {
  use super::*;
  use crate::world::World;

  #[test]
  fn query_mask_updating_with_component() -> Result<()> {
    let world = World::new();
    world.register_component::<u32>();
    world.register_component::<f32>();
    world.register_component::<usize>();

    let mut query = world.query();

    query.with_component::<u32>()?.with_component::<f32>()?.without_component::<usize>()?;

    assert_eq!(query.map, 3);
    Ok(())
  }

  #[test]
  fn get_component_works() -> Result<()> {
    let world = World::new();

    world.register_component::<u32>();
    world.register_component::<f32>();

    world.create_entity().with_component(100_u32)?;

    world.create_entity().with_component(10.0_f32)?;

    let mut query = world.query();

    let entities:Vec<QueryEntity> = query.with_component::<u32>()?.run();

    assert_eq!(entities.len(), 1);

    for entity in entities {
      assert_eq!(entity.id, 0);
      let health = entity.get_component::<u32>()?;
      assert_eq!(*health, 100);
    }
    Ok(())
  }

  #[test]
  fn query_for_entity_mutable() -> Result<()> {
    let world = World::new();
    world.register_component::<Health>().register_component::<f32>();

    world.create_entity().with_component(Health(100))?;

    world.create_entity().with_component(10.0_f32)?;

    let mut query = world.query();

    let entities:Vec<QueryEntity> = query.with_component::<Health>()?.run();

    assert_eq!(entities.len(), 1);

    for entity in entities {
      assert_eq!(entity.id, 0);
      let health = entity.get_component_mut::<Health>()?;
      assert_eq!(health.0, 100);
      health.0 += 1;
    }

    let entities:Vec<QueryEntity> = query.with_component::<Health>()?.run();

    for entity in entities {
      let health = entity.get_component::<Health>()?;
      assert_eq!(health.0, 101);
    }
    Ok(())
  }

  #[test]
  fn query_for_entity_after_component_delete() -> Result<()> {
    let world = World::new();
    world.register_component::<Health>();
    world.register_component::<Damage>();

    world.create_entity().with_component(Health(100))?;
    world.add_component(0, Damage(100))?;
    world.delete_component::<Damage>(0)?;

    let mut query = world.query();

    let entities = query.with_component::<Health>()?.with_component::<Damage>()?.run();
    assert_eq!(entities.len(), 0);
    Ok(())
  }

  #[test]
  fn query_for_entity_without_component() -> Result<()> {
    let world = World::new();
    world.register_component::<Health>();
    world.register_component::<Damage>();
    world.register_component::<usize>();
    world.register_component::<f32>();

    world.create_entity().with_component(Damage(100))?;
    world.create_entity().with_component(Damage(100))?.with_component(Health(100))?;
    world.create_entity().with_component(Health(30))?.with_component(5_usize)?;

    let mut query = world.query();
    let entities = query.with_component::<Health>()?.without_component::<Damage>()?.run();

    assert_eq!(entities.len(), 1);

    let entity = &entities[0];
    let health = entity.get_component::<Health>()?;
    assert_eq!(health.0, 30);

    Ok(())
  }
  struct Health(pub i32);
  struct Damage(pub u32);
}
