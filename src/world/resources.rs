use crate::{
  errors::EcsErrors,
  storage::{erased_collections::ErasedBox, type_info::TypeInfo, type_map::TypeMap, EcsData}
};
use std::cell::{Ref, RefCell, RefMut};

///Struct containing resources. Singleton values with only one instance in the
/// game world.
#[derive(Default)]
pub struct Resources {
  data:RefCell<TypeMap<ErasedBox>>
}

impl Resources {
  pub fn add_resource<T:EcsData>(&self, data:T) {
    let ty = TypeInfo::of::<T>();
    let data_vec = ErasedBox::new::<T>(data);
    self.data.borrow_mut().insert(ty, data_vec);
  }

  pub fn get<T:EcsData>(&self) -> Ref<T> {
    let ty:TypeInfo = TypeInfo::of::<T>();
    let borrowed_resource = self.data.borrow();

    Ref::map(borrowed_resource, |resource| {
      let data = resource
        .get(&ty)
        .ok_or(EcsErrors::ResourceDataDoesNotExist {
          component:ty.name().to_string()
        })
        .unwrap();
      data.get::<T>()
    })
  }

  pub fn get_mut<T:EcsData>(&self) -> RefMut<T> {
    let ty:TypeInfo = TypeInfo::of::<T>();
    let borrowed_resource = self.data.borrow_mut();

    RefMut::map(borrowed_resource, |resource| {
      let data = resource
        .get(&ty)
        .ok_or(EcsErrors::ResourceDataDoesNotExist {
          component:ty.name().to_string()
        })
        .unwrap();
      data.get_mut::<T>()
    })
  }

  pub fn remove<T:EcsData>(&self) {
    let ty:TypeInfo = TypeInfo::of::<T>();
    self.data.borrow_mut().remove(&ty);
  }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
  use super::*;
  #[test]
  fn add_resource() {
    let resources:Resources = init_resource();
    let binding = resources.data.borrow();
    let stored_resource = binding.get(&TypeInfo::of::<WorldWidth>()).unwrap();
    let extracted_world_width = stored_resource.get::<WorldWidth>();
    assert_eq!(extracted_world_width.0, 100.0)
  }

  #[test]
  fn get_resource() {
    let resources = init_resource();

    let world_width = resources.get::<WorldWidth>();
    assert_eq!(world_width.0, 100.0)
  }

  #[test]
  fn mut_get_resource() {
    let resources = init_resource();
    {
      let mut world_width = resources.get_mut::<WorldWidth>();
      world_width.0 += 1.0
    }
    let world_width = resources.get_mut::<WorldWidth>();
    assert_eq!(world_width.0, 101.0)
  }

  #[test]
  fn remove_resource() {
    let resources = init_resource();

    resources.remove::<WorldWidth>();
    let world_width_typeid = TypeInfo::of::<WorldWidth>();
    assert!(!resources.data.borrow().contains_key(&world_width_typeid));
  }

  fn init_resource() -> Resources {
    let resources:Resources = Resources::default();
    let world_width:WorldWidth = WorldWidth(100.0);

    resources.add_resource(world_width);

    return resources;
  }
  struct WorldWidth(pub f32);
}
