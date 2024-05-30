use super::{entities::Entity, World};
use crate::storage::{Bundle, EcsData, NoDropTuple, TypeInfo};

/// Records operations for future application to a World
///
/// Useful when operations cannot be applied directly due to ordering concerns
/// or borrow checking.
pub struct CommandBuffer(Vec<Command>);

impl CommandBuffer {
  pub fn new() -> Self {
    CommandBuffer(Default::default())
  }

  /// Create a new entity with the provided components.
  pub fn spawn_entity<B:Bundle>(&mut self, components:B) {
    let insert_info = InsertInfo {
      entity:None,
      components:NoDropTuple::new(components)
    };
    self.0.push(Command::InsertOrSpawn(insert_info))
  }

  /// Delete the specified entity.
  pub fn delete_entity(&mut self, entity:Entity) {
    self.0.push(Command::DeleteEntity(entity));
  }

  /// Add a commponent to the specified entity.
  pub fn insert_component<T:EcsData>(&mut self, entity:Entity, component:T) {
    self.insert_components(entity, (component,));
  }

  /// Add commponents to the specified entity. To insert one
  /// component see [`Self::insert_component`].
  pub fn insert_components<B:Bundle>(&mut self, entity:Entity, components:B) {
    let insert_info = InsertInfo {
      entity:Some(entity),
      components:NoDropTuple::new(components)
    };
    self.0.push(Command::InsertOrSpawn(insert_info))
  }

  /// Removes the component specified by the generic parameter.
  pub fn remove_component<T:EcsData>(&mut self, entity:Entity) {
    self.remove_components::<(T,)>(entity)
  }

  /// Removes the components specified by the generic parameter.
  ///
  /// Enter components as a [`Bundle`] i.e. `(A,B,C)`.
  pub fn remove_components<T:Bundle>(&mut self, entity:Entity) {
    let remove_info = RemoveInfo { entity, tys:T::types() };
    self.0.push(Command::RemoveComponent(remove_info))
  }

  /// Execute the buffered commands.
  pub fn run(&mut self, world:&mut World) {
    for cmd in &self.0 {
      match cmd {
        Command::InsertOrSpawn(insert_info) => {
          let entity = match insert_info.entity {
            Some(entity) => entity,
            None => world.reserve_entity()
          };
          for index in 0..insert_info.components.len() {
            let (ty, ptr) = insert_info.components.get(index);
            world.add_component_erased(entity, ty, ptr).unwrap();
          }
        }
        Command::RemoveComponent(remove_info) => {
          for ty in &remove_info.tys {
            world.delete_component_erased(remove_info.entity, *ty).unwrap();
          }
        }
        Command::DeleteEntity(entity) => world.delete_entity(*entity).unwrap()
      }
    }
  }

  /// Remove all commands from the [`CommandBuffer`].
  pub fn clear(&mut self) {
    *self = CommandBuffer::new();
  }
}

/// A buffered command
enum Command {
  InsertOrSpawn(InsertInfo),
  RemoveComponent(RemoveInfo),
  DeleteEntity(Entity)
}

struct RemoveInfo {
  entity:Entity,
  tys:Vec<TypeInfo>
}

struct InsertInfo {
  entity:Option<Entity>,
  components:NoDropTuple
}

#[cfg(test)]
mod tests {
  use crate::world::{command_buffer::CommandBuffer, World};

  //miri is still erroring but the test passes,
  // somehow the pointer to the string it tries to drop is incorrect (zero) unsure
  // how that happens as I believe it should invalidate other operations too...
  // only happens for vecs
  // Seems to only happen on dropping the world according to the backtrace.
  #[test]
  fn insert_into_entities() {
    let mut world = World::new();
    world
      .register_component::<bool>()
      .register_component::<String>()
      .register_component::<u32>()
      .register_component::<f32>();

    let mut buffer = CommandBuffer::new();

    buffer.spawn_entity((true, "a".to_string()));
    buffer.spawn_entity((1_u32, 1.0_f32));
    buffer.spawn_entity((true, "a".to_string()));
    buffer.spawn_entity((1.0_f32, "a".to_string()));
    buffer.run(&mut world);

    let bool_0 = world.get_component::<bool>(0).unwrap();
    let string_0 = world.get_component::<String>(0).unwrap();
    assert_eq!(*bool_0, true);
    assert_eq!(*string_0, "a".to_string());

    let u32_1 = world.get_component::<u32>(1).unwrap();
    let uf32_1 = world.get_component::<f32>(1).unwrap();
    assert_eq!(*u32_1, 1);
    assert_eq!(*uf32_1, 1.0);

    let bool_2 = world.get_component::<bool>(2).unwrap();
    let string_2 = world.get_component::<String>(2).unwrap();
    assert_eq!(*bool_2, true);
    assert_eq!(*string_2, "a".to_string());

    let f32_3 = world.get_component::<f32>(3).unwrap();
    let string_3 = world.get_component::<String>(3).unwrap();
    assert_eq!(*f32_3, 1.0);
    assert_eq!(*string_3, "a".to_string());
  }
}
