use std::{alloc::Layout, ptr::NonNull};

use crate::storage::{Bundle, EcsData, TypeInfo};

use super::{entities::Entity, World};

/// Records operations for future application to a World
///
/// Useful when operations cannot be applied directly due to ordering concerns
/// or borrow checking.
pub struct CommandBuffer {
  commands:Vec<Command>,
  storage:NonNull<u8>,
  layout:Layout,
  cursor:usize,
  // components:Vec<ComponentInfo>,
  ids:Vec<TypeInfo>
}

impl CommandBuffer {
  pub fn new() -> Self {
    CommandBuffer {
      commands:Default::default(),
      storage:NonNull::dangling(),
      layout:Layout::from_size_align(0, 8).unwrap(),
      cursor:0,
      ids:Vec::new()
    }
  }

  pub fn spawn_entity<T:Bundle>(&mut self, components:T) {
    let mut insert_info = InsertInfo {
      entity:None,
      components:Vec::new()
    };
    unsafe {
      components
        .put(|ptr, ty| {
          insert_info.components.push((ty, ptr));
          Ok(())
        })
        .unwrap();
    }
    self.commands.push(Command::InsertOrSpawn(insert_info))
  }

  pub fn delete_entity(&mut self, entity:Entity) {
    self.commands.push(Command::DeleteEntity(entity));
  }

  pub fn insert_component<T:EcsData>(&mut self, entity:Entity, component:T) {
    self.insert_components(entity, (component,));
  }

  pub fn insert_components<T:Bundle>(&mut self, entity:Entity, components:T) {
    let mut insert_info = InsertInfo {
      entity:Some(entity),
      components:Vec::new()
    };
    unsafe {
      components
        .put(|ptr, ty| {
          insert_info.components.push((ty, ptr));
          Ok(())
        })
        .unwrap();
    }
    self.commands.push(Command::InsertOrSpawn(insert_info))
  }

  /// Removes the component specified by the generic parameter.
  pub fn remove_component<T:EcsData>(&mut self, entity:Entity) {
    self.remove_components::<(T,)>(entity)
  }

  /// Removes the components specified by the generic parameter.
  ///
  /// Enter Components as a [`Bundle`].
  pub fn remove_components<T:Bundle>(&mut self, entity:Entity) {
    let remove_info = RemoveInfo { entity, tys:T::types() };
    self.commands.push(Command::RemoveComponent(remove_info))
  }

  pub fn run(&mut self, world:&mut World) {
    for cmd in &self.commands {
      match cmd {
        Command::InsertOrSpawn(insert_info) => {
          let entity = match insert_info.entity {
            Some(entity) => entity,
            None => world.reserve_entity()
          };
          for (ty, ptr) in &insert_info.components {
            world.add_component_erased(entity, *ty, *ptr).unwrap();
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

  /// Removes all commands from the [`CommandBuffer`].
  pub fn clear(&mut self) {}
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
  components:Vec<(TypeInfo, *mut u8)>
}

#[cfg(test)]
mod tests {
  use crate::{
    storage::{Bundle, TypeInfo},
    world::{command_buffer::CommandBuffer, World}
  };

  #[test]
  fn get_type_info_from_bundle() {
    let bundle_tys = <(u32, f32, String)>::types();
    assert_eq!(bundle_tys[0], TypeInfo::of::<u32>());
    assert_eq!(bundle_tys[1], TypeInfo::of::<f32>());
    assert_eq!(bundle_tys[2], TypeInfo::of::<String>());
  }

  #[test]
  fn insert_into_entities() {
    let mut world = World::new();
    world
      .register_component::<bool>()
      .register_component::<String>()
      .register_component::<u32>()
      .register_component::<f32>();

    let mut buffer = CommandBuffer::new();
    let ent = world.reserve_entity();
    let enta = world.reserve_entity();
    let entb = world.reserve_entity();
    let entc = world.reserve_entity();
    buffer.insert_components(ent, (true, "a".to_string()));
    buffer.insert_components(entc, (false, "a".to_string()));
    buffer.insert_components(enta, (1_u32, 1.0_f32));
    buffer.insert_components(entb, (1.0_f32, "a".to_string()));
    // world.add_components(ent, (true, "a".to_string()));
    // world.add_components(entc, (false, "a".to_string()));
    // world.add_components(enta, (1_u32, 1.0_f32));
    // world.add_components(entb, (1.0_f32, "a".to_string()));
    buffer.run(&mut world);

    let bool_1 = world.get_component::<bool>(ent).unwrap();
    let string_1 = world.get_component::<String>(ent).unwrap();
    assert_eq!(*bool_1, true);
    assert_eq!(*string_1, "a".to_string());

    let u32_a = world.get_component::<u32>(enta).unwrap();
    let f32_a = world.get_component::<f32>(enta).unwrap();
    assert_eq!(*u32_a, 1);
    assert_eq!(*f32_a, 1.0_f32);

    // let f32_b = world.get_component::<f32>(entb).unwrap();
    // let string_b = world.get_component::<String>(entb).unwrap();
    // assert_eq!(*f32_b, 1.0_f32);
    // assert_eq!(*string_b, "a".to_string());

    // let bool_c = world.get_component::<bool>(entc).unwrap();
    // let string_c = world.get_component::<String>(entc).unwrap();
    // assert_eq!(*bool_c, false);
    // assert_eq!(*string_c, "a".to_string());
  }
}
