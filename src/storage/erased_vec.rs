use crate::errors::ErasedVecErrors::{DoesNotContainType, ErasedVecAllocError, ErasedVecCapacityOverflow, IncorrectTypeInsertion, IndexOutOfBounds};
use std::{
  alloc,
  marker::PhantomData,
  ops::{Deref, DerefMut},
  ptr::{self, NonNull},
  slice
};

use super::type_info::TypeInfo;

// Refactor:
// -No `pop` method. Unsure it is needed.
// -No `remove` method. Unsure it is needed.
// -Rework the typed insert and pushes
// -I don't think Drop needs to drop each element individually
// -Drop can use the generic
// -Might need to add explict drop logic in pushing/insertion?

struct RawErasedVec<T> {
  ptr:NonNull<u8>,
  cap:usize,
  ty:TypeInfo,
  _data:PhantomData<T>
}

impl<T:'static + Send + Sync> RawErasedVec<T> {
  fn new() -> Self {
    let ty = TypeInfo::of::<T>();
    let cap = if ty.size() == 0 { usize::MAX } else { 0 };
    let ptr = NonNull::new(ty.size() as *mut u8).unwrap();

    RawErasedVec {
      ptr,
      cap,
      ty,
      _data:PhantomData
    }
  }

  fn grow(&mut self) {
    // since we set the capacity to usize::MAX when `ty` has size 0,
    // getting to here necessarily means the Vec is overfull.
    assert!(self.ty.size() != 0, "{ErasedVecCapacityOverflow}");

    let (new_cap, new_layout) = if self.cap == 0 {
      (1, self.ty.array(1).unwrap())
    } else {
      let new_cap = 2 * self.cap;
      let new_layout = self.ty.array(new_cap).unwrap();
      (new_cap, new_layout)
    };

    // Ensure that the new allocation doesn't exceed `isize::MAX` bytes.
    assert!(new_layout.size() <= isize::MAX as usize, "{ErasedVecAllocError}",);

    let new_ptr = if self.cap == 0 {
      unsafe { alloc::alloc(new_layout) }
    } else {
      let old_ptr = self.ptr.as_ptr();
      let old_layout = self.ty.array(self.cap).unwrap();
      unsafe { alloc::realloc(old_ptr, old_layout, new_layout.size()) }
    };

    // If allocation fails, `new_ptr` will be null, in which case we abort.
    self.ptr = match NonNull::new(new_ptr) {
      Some(p) => p,
      None => alloc::handle_alloc_error(new_layout)
    };

    self.cap = new_cap;
  }
}

// impl Drop for RawErasedVec {
//   fn drop(&mut self) {
//     if self.cap != 0 && self.ty.size() != 0 {
//       let layout = self.ty.array(self.cap).unwrap();
//       unsafe { alloc::dealloc(self.ptr.as_ptr(), layout) }
//     }
//   }
// }

///A type erased vector used for storing data in the ECS.
pub struct ErasedVec<T> {
  buf:RawErasedVec<T>,
  len:usize
}

impl<T:'static + Send + Sync> ErasedVec<T> {
  ///Constructs a new, empty [`ErasedVec<T>`].
  ///
  ///The vector will not allocate until elements are pushed onto it.
  pub fn new() -> Self {
    ErasedVec {
      buf:RawErasedVec::new(),
      len:0
    }
  }

  fn ptr(&self) -> *mut u8 {
    self.buf.ptr.as_ptr()
  }

  fn ty(&self) -> TypeInfo {
    self.buf.ty
  }

  fn cap(&self) -> usize {
    self.buf.cap
  }

  ///Append a value to the back of the [`ErasedVec`].
  pub fn push(&mut self, value:T) {
    // Grow the Vec if it is at max capacity
    if self.len == self.cap() {
      self.buf.grow()
    }

    // Copy the value as raw bits into the `ErasedVec`
    let val_ptr = (&value as *const T).cast::<u8>();

    unsafe {
      let offset = self.len * self.ty().size();
      ptr::copy_nonoverlapping(val_ptr, self.ptr().add(offset), self.ty().size());
    }

    self.len += 1;
  }

  ///Append a type-erased value to the back of the [`ErasedVec`].
  ///
  /// # Panics
  ///
  /// Panics if the [`TypeInfo`] of the value does not match the type contained
  /// in the `ErasedVec`.
  pub fn push_erased(&mut self, val_ptr:*mut u8, ty:TypeInfo) {
    // Grow the Vec if it is at max capacity
    if self.len == self.cap() {
      self.buf.grow()
    }

    // Confirm the inserted value is the correct type.
    self.assert_type_info_insert(ty);

    // Copy the value as raw bits into the `ErasedVec`
    unsafe {
      let offset = self.len * self.ty().size();
      ptr::copy_nonoverlapping(val_ptr, self.ptr().add(offset), self.ty().size());
    }

    self.len += 1;
  }

  /// Inserts an element at position `index` within the vector, shifting all
  /// elements after it to the right.
  ///
  /// # Panics
  ///
  /// Panics if `index > len`.
  pub fn insert(&mut self, index:usize, value:T) {
    // Check whether the index is within bounds
    assert!(index <= self.len, "{}", IndexOutOfBounds { len:self.len, index });
    if self.len == self.cap() {
      self.buf.grow()
    }

    unsafe {
      let start_offset = index * self.ty().size();
      let end_offset = (index + 1) * self.ty().size();
      let count = (self.len - index) * self.ty().size();
      ptr::copy(self.ptr().add(start_offset), self.ptr().add(end_offset), count);

      // Copy the value as raw bits into the `ErasedVec`
      let val_ptr = (&value as *const T).cast::<u8>();
      ptr::copy_nonoverlapping(val_ptr, self.ptr().add(start_offset), self.ty().layout().size());
    }

    self.len += 1;
  }

  /// Inserts an element at position `index` within the vector, shifting all
  /// elements after it to the right.
  ///
  /// # Panics
  ///
  /// Panics if `index > len`.
  ///
  /// Panics if `ty` != `self.ty()`
  pub fn insert_erased(&mut self, val_ptr:*mut u8, ty:TypeInfo, index:usize) {
    // Check whether the index is within bounds
    assert!(index <= self.len, "{}", IndexOutOfBounds { len:self.len, index });
    if self.len == self.cap() {
      self.buf.grow()
    }

    self.assert_type_info_insert(ty);

    unsafe {
      let start_offset = index * self.ty().size();
      let end_offset = (index + 1) * self.ty().size();
      let count = (self.len - index) * self.ty().size();
      ptr::copy(self.ptr().add(start_offset), self.ptr().add(end_offset), count);

      // Copy the value as raw bits into the `ErasedVec`
      ptr::copy_nonoverlapping(val_ptr, self.ptr().add(start_offset), self.ty().layout().size());
    }

    self.len += 1;
  }

  ///Panics if the queried [`TypeInfo`] is not the same as the data the
  /// [`ErasedVec`] holds.
  fn assert_type_info_insert(&self, ty:TypeInfo) {
    assert_eq!(
      ty,
      self.ty(),
      "{}",
      IncorrectTypeInsertion {
        insert_type:ty.name(),
        vec_type:self.ty().name()
      }
    );
  }

  ///Panics if the queried [`TypeInfo`] is not the same as the data the
  /// [`ErasedVec`] holds.
  fn assert_type_info(&self, ty:TypeInfo) {
    assert_eq!(ty, self.ty(), "{}", DoesNotContainType(ty.name()));
  }
}

// impl Drop for ErasedVec {
//   fn drop(&mut self) {
//     if self.cap() != 0 {
//       // Drop the elements inside the `ErasedVec`
//       for i in 0..=self.len {
//         let offset = i * self.ty().size();
//         unsafe {
//           let data_ptr = self.ptr().add(offset);
//           self.ty().drop(data_ptr)
//         }
//       }
//       // Deallocate the buffer
//       let layout = self.ty().array(self.cap()).unwrap();
//       unsafe { alloc::dealloc(self.ptr(), layout) }
//     }
//   }
// }

impl<T:'static + Send + Sync> Deref for ErasedVec<T> {
  type Target = [T];

  fn deref(&self) -> &Self::Target {
    let len = self.len * self.ty().size();
    unsafe { slice::from_raw_parts(self.ptr() as *mut T, len) }
  }
}

impl<T:'static + Send + Sync> DerefMut for ErasedVec<T> {
  fn deref_mut(&mut self) -> &mut [T] {
    let len = self.len * self.ty().size();
    unsafe { slice::from_raw_parts_mut(self.ptr() as *mut T, len) }
  }
}

#[cfg(test)]
mod test {
  use crate::storage::type_info::TypeInfo;

  use super::ErasedVec;

  #[test]
  fn push_into_and_read() {
    let health_1 = Health::new(100);
    let health_2 = Health::new(5483392);
    let health_3 = Health::new(25);

    let mut heath_vec = ErasedVec::new();
    heath_vec.push(health_1);
    heath_vec.push(health_2);
    heath_vec.push(health_3);

    //Checking pushing normally works
    pull_and_check(&heath_vec);
    pull_and_mut(&mut heath_vec);
    let health_3 = heath_vec[2].min;
    assert_eq!(health_3, 6);

    let ty = TypeInfo::of::<Health>();
    let mut health_1 = Health::new(100);
    let mut health_2 = Health::new(5483392);
    let mut health_3 = Health::new(25);

    let mut heath_vec:ErasedVec<Health> = ErasedVec::new();
    heath_vec.push_erased((&mut health_1 as *mut Health).cast::<u8>(), ty);
    heath_vec.push_erased((&mut health_2 as *mut Health).cast::<u8>(), ty);
    heath_vec.push_erased((&mut health_3 as *mut Health).cast::<u8>(), ty);

    //Checking pushing erased normally works
    pull_and_check(&heath_vec);
    pull_and_mut(&mut heath_vec);
    let health_3 = heath_vec[2].min;
    assert_eq!(health_3, 6);

    fn pull_and_check(vec:&ErasedVec<Health>) {
      let retrieved_health = vec[0];
      let retrieved_health_2 = vec[1];
      let retrieved_health_3 = vec[2];

      assert_eq!(retrieved_health.max, 100);
      assert_eq!(retrieved_health_2.max, 5483392);
      assert_eq!(retrieved_health_3.max, 25);
    }

    fn pull_and_mut(vec:&mut ErasedVec<Health>) {
      vec[2].min = 6;
    }
  }

  #[test]
  fn push_zst_into_and_read() {
    let mut player_vec:ErasedVec<Option<Player>> = ErasedVec::new();
    player_vec.push(Some(Player));
    player_vec.push(Some(Player));
    player_vec.push(None);
    player_vec.push(Some(Player));
    player_vec.push(None);
    player_vec.push(None);

    //Confirm pushing normally works
    assert_eq!(player_vec.len(), 6);
    assert_eq!(player_vec[0], Some(Player));
    assert_eq!(player_vec[1], Some(Player));
    assert_eq!(player_vec[2], None);
    assert_eq!(player_vec[3], Some(Player));
    assert_eq!(player_vec[4], None);
    assert_eq!(player_vec[5], None);

    let mut player_vec:ErasedVec<Option<Player>> = ErasedVec::new();
    let ty:TypeInfo = TypeInfo::of::<Option<Player>>();
    player_vec.push_erased((&mut Some(Player) as *mut Option<Player>).cast::<u8>(), ty);
    player_vec.push_erased((&mut Some(Player) as *mut Option<Player>).cast::<u8>(), ty);
    player_vec.push_erased((&mut None as *mut Option<Player>).cast::<u8>(), ty);
    player_vec.push_erased((&mut Some(Player) as *mut Option<Player>).cast::<u8>(), ty);
    player_vec.push_erased((&mut None as *mut Option<Player>).cast::<u8>(), ty);
    player_vec.push_erased((&mut None as *mut Option<Player>).cast::<u8>(), ty);

    //Confirm pushing erased works
    assert_eq!(player_vec.len(), 6);
    assert_eq!(player_vec[0], Some(Player));
    assert_eq!(player_vec[1], Some(Player));
    assert_eq!(player_vec[2], None);
    assert_eq!(player_vec[3], Some(Player));
    assert_eq!(player_vec[4], None);
    assert_eq!(player_vec[5], None);
  }

  #[test]
  fn push_collection_into_and_read() {
    //Check pushing normally works
    let mut path_vec:ErasedVec<Path> = ErasedVec::new();
    let path_1 = Path::new(vec![[0.0, 9222.444], [3.432, 5933.9999999], [3.484, 19444.333]]);
    let path_2 = Path::new(vec![[222222.22222, 5933.9999999]]);
    let path_3 = Path::new(Vec::default());

    path_vec.push(path_1);
    path_vec.push(path_2);
    path_vec.push(path_3);

    let retrieved_path_1 = &path_vec[0];
    assert_eq!(retrieved_path_1.steps[0][0], 0.0);
    assert_eq!(retrieved_path_1.steps[1][0], 3.432);

    let retrieved_path_2 = &path_vec[1];
    assert_eq!(retrieved_path_2.steps[0][0], 222222.22222);
    assert_eq!(retrieved_path_2.steps[0][1], 5933.9999999);

    //Check pushing erased works
    let mut path_vec:ErasedVec<Path> = ErasedVec::new();
    let mut path_1 = Path::new(vec![[0.0, 9222.444], [3.432, 5933.9999999], [3.484, 19444.333]]);
    let mut path_2 = Path::new(vec![[222222.22222, 5933.9999999]]);
    let mut path_3 = Path::new(Vec::default());
    let ty = TypeInfo::of::<Path>();

    path_vec.push_erased((&mut path_1 as *mut Path).cast::<u8>(), ty);
    path_vec.push_erased((&mut path_2 as *mut Path).cast::<u8>(), ty);
    path_vec.push_erased((&mut path_3 as *mut Path).cast::<u8>(), ty);

    let retrieved_path_1 = &path_vec[0];
    assert_eq!(retrieved_path_1.steps[0][0], 0.0);
    assert_eq!(retrieved_path_1.steps[1][0], 3.432);

    let retrieved_path_2 = &path_vec[1];
    assert_eq!(retrieved_path_2.steps[0][0], 222222.22222);
    assert_eq!(retrieved_path_2.steps[0][1], 5933.9999999);
  }

  #[test]
  fn inserting_into_works() {
    let mut health_vec:ErasedVec<Health> = ErasedVec::new();
    let health_1 = Health::new(100);
    let health_2 = Health::new(5483392);
    let health_3 = Health::new(25);
    let health_4 = Health::new(30);

    health_vec.push(health_1);
    health_vec.push(health_2);
    health_vec.push(health_3);

    //Insert normally and c heck the values
    health_vec.insert(1, health_4);
    assert_eq!(health_vec[0].min, health_1.min);
    assert_eq!(health_vec[1].min, health_4.min);
    assert_eq!(health_vec[2].min, health_2.min);
    assert_eq!(health_vec[3].min, health_3.min);

    let mut health_vec:ErasedVec<Health> = ErasedVec::new();
    let health_1 = Health::new(100);
    let health_2 = Health::new(5483392);
    let health_3 = Health::new(25);
    let mut health_4 = Health::new(30);

    health_vec.push(health_1);
    health_vec.push(health_2);
    health_vec.push(health_3);

    //Insert erased and c heck the values
    health_vec.insert_erased((&mut health_4 as *mut Health).cast::<u8>(), TypeInfo::of::<Health>(), 1);
    assert_eq!(health_vec[0].min, health_1.min);
    assert_eq!(health_vec[1].min, health_4.min);
    assert_eq!(health_vec[2].min, health_2.min);
    assert_eq!(health_vec[3].min, health_3.min);
  }

  #[derive(Debug, PartialEq, PartialOrd)]
  struct Player;

  #[derive(Debug, Clone, Copy)]
  struct Health {
    pub max:i32,
    pub min:i32
  }

  impl Health {
    pub fn new(max:i32) -> Self {
      Health { max, min:max }
    }
  }

  struct Path {
    steps:Vec<[f32; 2]>
  }

  impl Path {
    fn new(steps:Vec<[f32; 2]>) -> Self {
      Path { steps }
    }
  }
}
