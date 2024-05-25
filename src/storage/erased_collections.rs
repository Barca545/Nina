use super::type_info::TypeInfo;
use crate::errors::ErasedVecErrors::{DoesNotContainType, ErasedVecAllocError, ErasedVecCapacityOverflow, IncorrectTypeInsertion, IndexOutOfBounds};
use std::{
  alloc, mem,
  ptr::{self, NonNull}
};

// Refactor:
// -No `pop` method. Unsure it is needed.
// -No `remove` method. Unsure it is needed.
// -Rename this collections

struct RawErasedVec {
  ptr:NonNull<u8>,
  cap:usize,
  ty:TypeInfo
}

impl RawErasedVec {
  fn new<T:'static>() -> Self {
    let ty = TypeInfo::of::<T>();
    let cap = if ty.size() == 0 { usize::MAX } else { 0 };
    let ptr = NonNull::dangling();

    RawErasedVec { ptr, cap, ty }
  }
  fn grow_exact(&mut self, cap:usize) {
    // since we set the capacity to usize::MAX when `ty` has size 0,
    // getting to here necessarily means the Vec is overfull.
    assert!(self.ty.size() != 0, "{ErasedVecCapacityOverflow}");

    let (new_cap, new_layout) = if self.cap == 0 {
      (1, self.ty.array(1).unwrap())
    } else {
      let new_cap = cap;
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

  fn grow(&mut self) {
    self.grow_exact(2 * self.cap);
  }
}

impl Drop for RawErasedVec {
  fn drop(&mut self) {
    if self.cap != 0 && self.ty.size() != 0 {
      // unsafe { self.ty.drop(self.ptr.as_ptr()) }

      // Deallocate the buffer
      let layout = self.ty.array(self.cap).unwrap();
      unsafe { alloc::dealloc(self.ptr.as_ptr(), layout) }
    }
  }
}

///A type erased vector used for storing data in the ECS.
pub struct ErasedVec {
  buf:RawErasedVec,
  len:usize
}

impl ErasedVec {
  ///Constructs a new, empty [`ErasedVec<T>`].
  ///
  ///The vector will not allocate until elements are pushed onto it.
  pub fn new<T:'static>() -> Self {
    ErasedVec {
      buf:RawErasedVec::new::<T>(),
      len:0
    }
  }

  fn ptr(&self) -> *mut u8 {
    self.buf.ptr.as_ptr()
  }

  /// Returns a ptr to the value stored at the requested index.
  ///
  /// # Warning
  ///
  /// The pointer is calculated using the internal [`TypeInfo`].
  pub unsafe fn indexed_ptr<T:'static>(&self, index:usize) -> *mut T {
    let index = index * self.ty().size();
    self.ptr().add(index) as *mut T
  }

  fn ty(&self) -> TypeInfo {
    self.buf.ty
  }

  fn cap(&self) -> usize {
    self.buf.cap
  }

  ///Returns the number of elements in the vector, also referred to as its
  /// ‘length’.
  pub fn len(&self) -> usize {
    self.len
  }

  ///Fetch data from the [`ErasedVec`] by index.
  ///
  /// # Panics
  ///
  /// Panics if the [`TypeInfo`] of the value does not match the type contained
  /// in the `ErasedVec`.
  ///
  /// Panics if `index` > `self.len`.
  pub fn get<T:'static>(&self, index:usize) -> &T {
    // Confirm the vector contains `T`
    self.assert_type_info(TypeInfo::of::<T>());

    // Confirm the index is in bounds
    assert!(index <= self.len, "{}", IndexOutOfBounds { len:self.len, index });

    // Get a pointer the data and cast it to `&T`
    unsafe { &*(self.indexed_ptr(index)) }
  }

  ///Fetch data from the [`ErasedVec`] by index.
  ///
  /// # Warning
  ///
  /// Does not check whether the `ErasedVec` contains the requested type `T`.
  ///
  /// # Panics
  ///
  /// Panics if `index` > `self.len`.
  pub unsafe fn get_unchecked<T:'static + Send + Sync>(&self, index:usize) -> &T {
    // Confirm the index is in bounds
    assert!(index <= self.len, "{}", IndexOutOfBounds { len:self.len, index });

    // Get a pointer the data and cast it to `&T`
    unsafe { &*(self.indexed_ptr(index)) }
  }

  ///Fetch data mutably from the [`ErasedVec`] by index.
  ///
  /// # Panics
  ///
  /// Panics if the [`TypeInfo`] of the value does not match the type contained
  /// in the `ErasedVec`.
  ///
  /// Panics if `index` > `self.len`.
  pub fn get_mut<T:'static>(&self, index:usize) -> &mut T {
    // Confirm the vector contains `T`
    self.assert_type_info(TypeInfo::of::<T>());

    // Confirm the index is in bounds
    assert!(index <= self.len, "{}", IndexOutOfBounds { len:self.len, index });

    // Get a pointer the data and cast it to `&mut T`
    unsafe { &mut *(self.indexed_ptr(index)) }
  }

  ///Fetch data mutably sfrom the [`ErasedVec`] by index.
  ///
  /// # Warning
  ///
  /// Does not check whether the `ErasedVec` contains the requested type `T`.
  ///
  /// # Panics
  ///
  /// Panics if `index` > `self.len`.
  pub unsafe fn get_mut_unchecked<T:'static + Send + Sync>(&self, index:usize) -> &mut T {
    // Confirm the index is in bounds
    assert!(index <= self.len, "{}", IndexOutOfBounds { len:self.len, index });

    // Get a pointer the data and cast it to `&T`
    unsafe { &mut *(self.indexed_ptr(index)) }
  }

  ///Pushes a value semantically equivelent to `None<T>` into the
  /// [`ErasedVec`].
  ///
  /// # Warning
  ///
  /// Data is padded with 0s, attempting to access it before it is overwritten
  /// with a value of type `T` will cause undefined behavior.
  pub fn pad(&mut self) {
    let mut padding:Vec<u8> = Vec::new();
    padding.resize(self.ty().size(), 0);
    let padding = padding.as_mut_ptr();

    self.push_erased(padding, self.ty())
  }

  ///Append a value to the back of the [`ErasedVec`].
  pub fn push<T:'static + Send + Sync>(&mut self, value:T) {
    self.assert_type_info_insert(TypeInfo::of::<T>());

    // Grow the Vec if it is at max capacity
    if self.len == self.cap() {
      self.buf.grow()
    }

    // Copy the value as raw bits into the `ErasedVec`
    // let value = ManuallyDrop::new(value);s
    // let val_ptr = (&value as *const ManuallyDrop<T>).cast::<u8>();
    let val_ptr = (&value as *const T).cast::<u8>();

    unsafe {
      let offset = self.len * self.ty().size();
      ptr::copy_nonoverlapping(val_ptr, self.ptr().add(offset), self.ty().size());
    }

    mem::forget(value);

    self.len += 1;
  }

  ///Append a type-erased value to the back of the [`ErasedVec`].
  ///
  /// # Warning
  ///
  /// Must call [`mem::forget`] on the value being inserted or a double free
  /// will occur.
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
  pub fn insert<T:'static + Send + Sync>(&mut self, index:usize, value:T) {
    self.assert_type_info_insert(TypeInfo::of::<T>());

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
      ptr::copy_nonoverlapping(val_ptr, self.ptr().add(start_offset), self.ty().size());
    }

    self.len += 1;
  }

  /// Inserts an element at position `index` within the vector, shifting all
  /// elements after it to the right.
  ///
  /// # Warning
  ///
  /// Must call [`mem::forget`] on the value being inserted or a double free
  /// will occur.
  ///
  /// # Panics
  ///
  /// Panics if `index > len`.
  ///
  /// Panics if `ty` != `self.ty()`
  pub fn insert_erased(&mut self, val_ptr:*mut u8, ty:TypeInfo, index:usize) {
    if self.len == self.cap() {
      self.buf.grow()
    }

    // Check whether the index is within bounds
    assert!(index <= self.len, "{}", IndexOutOfBounds { len:self.len, index });

    self.assert_type_info_insert(ty);

    unsafe {
      let start_offset = index * self.ty().size();
      let end_offset = (index + 1) * self.ty().size();
      let count = (self.len - index) * self.ty().size();
      ptr::copy(self.ptr().add(start_offset), self.ptr().add(end_offset), count);

      // Copy the value as raw bits into the `ErasedVec`
      ptr::copy_nonoverlapping(val_ptr, self.ptr().add(start_offset), self.ty().size());
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

impl Drop for ErasedVec {
  fn drop(&mut self) {
    for index in 0..self.len {
      unsafe { self.ty().drop(self.indexed_ptr(index)) }
    }
  }
}

pub struct ErasedBox(RawErasedVec);

impl ErasedBox {
  pub fn new<T:'static>(value:T) -> Self {
    // Create the buf
    let mut buf = RawErasedVec::new::<T>();
    buf.grow_exact(1);

    // Allocate space in the buf and insert the data into it
    unsafe {
      // Copy the value as raw bits into the `RawErasedVec` buf
      let val_ptr = (&value as *const T).cast::<u8>();
      ptr::copy_nonoverlapping(val_ptr, buf.ptr.as_ptr(), buf.ty.size());
    }

    mem::forget(value);

    ErasedBox(buf)
  }

  fn ptr(&self) -> *mut u8 {
    self.0.ptr.as_ptr()
  }

  fn ty(&self) -> TypeInfo {
    self.0.ty
  }

  ///Fetch [`ErasedBox`]'s data.
  ///
  /// # Panics
  ///
  /// Panics if the [`TypeInfo`] of the value does not match the type contained
  /// in the `ErasedVec`.
  ///
  /// Panics if `index` > `self.len`
  pub fn get<T:'static>(&self) -> &T {
    // Confirm the vector contains `T`
    self.assert_type_info(TypeInfo::of::<T>());

    // Get a pointer the data and cast it to `&T`;
    unsafe { &*(self.ptr() as *const T) }
  }

  ///Fetch [`ErasedBox`]'s data mutably.
  ///
  /// # Panics
  ///
  /// Panics if the [`TypeInfo`] of the value does not match the type contained
  /// in the `ErasedVec`.
  ///
  /// Panics if `index` > `self.len`
  pub fn get_mut<T:'static>(&self) -> &mut T {
    // Confirm the vector contains `T`
    self.assert_type_info(TypeInfo::of::<T>());

    // Get a pointer the data and cast it to `&mut T`
    unsafe { &mut *(self.ptr() as *mut T) }
  }

  ///Panics if the queried [`TypeInfo`] is not the same as the data the
  /// [`ErasedVec`] holds.
  fn assert_type_info(&self, ty:TypeInfo) {
    assert_eq!(ty, self.ty(), "{}", DoesNotContainType(ty.name()));
  }
}

impl Drop for ErasedBox {
  fn drop(&mut self) {
    // Drop the data
    unsafe { self.ty().drop(self.ptr()) }
  }
}

#[cfg(test)]
mod test {
  use super::ErasedVec;
  use crate::storage::type_info::TypeInfo;

  #[test]
  fn push_into_and_read() {
    let health_1 = Health::new(100);
    let health_2 = Health::new(5483392);
    let health_3 = Health::new(25);

    let mut heath_vec = ErasedVec::new::<Health>();
    heath_vec.push(health_1);
    heath_vec.push(health_2);
    heath_vec.push(health_3);

    //Checking pushing normally works
    pull_and_check(&heath_vec);
    pull_and_mut(&mut heath_vec);
    let health_3 = heath_vec.get::<Health>(2).min;
    assert_eq!(health_3, 6);

    let ty = TypeInfo::of::<Health>();
    let mut health_1 = Health::new(100);
    let mut health_2 = Health::new(5483392);
    let mut health_3 = Health::new(25);

    let mut heath_vec = ErasedVec::new::<Health>();
    heath_vec.push_erased((&mut health_1 as *mut Health).cast::<u8>(), ty);
    heath_vec.push_erased((&mut health_2 as *mut Health).cast::<u8>(), ty);
    heath_vec.push_erased((&mut health_3 as *mut Health).cast::<u8>(), ty);

    //Checking pushing erased normally works
    pull_and_check(&heath_vec);
    pull_and_mut(&mut heath_vec);
    let health_3 = heath_vec.get::<Health>(2).min;
    assert_eq!(health_3, 6);

    fn pull_and_check(vec:&ErasedVec) {
      let retrieved_health = vec.get::<Health>(0);
      let retrieved_health_2 = vec.get::<Health>(1);
      let retrieved_health_3 = vec.get::<Health>(2);

      assert_eq!(retrieved_health.max, 100);
      assert_eq!(retrieved_health_2.max, 5483392);
      assert_eq!(retrieved_health_3.max, 25);
    }

    fn pull_and_mut(vec:&mut ErasedVec) {
      vec.get_mut::<Health>(2).min = 6;
    }
  }

  #[test]
  fn push_zst_into_and_read() {
    let mut player_vec = ErasedVec::new::<Player>();
    player_vec.push(Player);
    player_vec.push(Player);
    player_vec.pad();
    player_vec.push(Player);
    player_vec.pad();
    player_vec.pad();

    //Confirm pushing normally works
    assert_eq!(player_vec.len, 6);

    assert_eq!(*player_vec.get::<Player>(0), Player);
    assert_eq!(*player_vec.get::<Player>(1), Player);
    assert_eq!(unsafe { *player_vec.get_unchecked::<[u8; 0]>(2) }, []);
    assert_eq!(*player_vec.get::<Player>(3), Player);
    assert_eq!(unsafe { *player_vec.get_unchecked::<[u8; 0]>(4) }, []);
    assert_eq!(unsafe { *player_vec.get_unchecked::<[u8; 0]>(5) }, []);

    let mut player_vec = ErasedVec::new::<Player>();
    let ty:TypeInfo = TypeInfo::of::<Player>();
    player_vec.push_erased((&mut Player as *mut Player).cast::<u8>(), ty);
    player_vec.push_erased((&mut Player as *mut Player).cast::<u8>(), ty);
    player_vec.pad();
    player_vec.push_erased((&mut Player as *mut Player).cast::<u8>(), ty);
    player_vec.pad();
    player_vec.pad();

    //Confirm pushing erased works
    assert_eq!(player_vec.len, 6);
    assert_eq!(player_vec.len, 6);
    assert_eq!(player_vec.len, 6);

    assert_eq!(*player_vec.get::<Player>(0), Player);
    assert_eq!(*player_vec.get::<Player>(1), Player);
    assert_eq!(unsafe { *player_vec.get_unchecked::<[u8; 0]>(2) }, []);
    assert_eq!(*player_vec.get::<Player>(3), Player);
    assert_eq!(unsafe { *player_vec.get_unchecked::<[u8; 0]>(4) }, []);
    assert_eq!(unsafe { *player_vec.get_unchecked::<[u8; 0]>(5) }, []);
  }

  //This is the source of the error
  //Something with the drop logic of collections is the problem
  #[test]
  fn push_collection_into_and_read() {
    //Check pushing normally works
    let mut path_vec = ErasedVec::new::<Path>();
    let path_1 = Path::new(vec![[0.0, 9222.444], [3.432, 5933.9999999], [3.484, 19444.333]]);
    let path_2 = Path::new(vec![[222222.22222, 5933.9999999]]);
    let path_3 = Path::new(Vec::default());

    path_vec.push(path_1);
    path_vec.push(path_2);
    path_vec.push(path_3);

    let retrieved_path_1 = path_vec.get::<Path>(0);
    assert_eq!(retrieved_path_1.steps[0][0], 0.0);
    assert_eq!(retrieved_path_1.steps[1][0], 3.432);

    let retrieved_path_2 = path_vec.get::<Path>(1);
    assert_eq!(retrieved_path_2.steps[0][0], 222222.22222);
    assert_eq!(retrieved_path_2.steps[0][1], 5933.9999999);

    //Check pushing erased works
    // let mut path_vec = ErasedVec::new::<Path>();
    // let mut path_1 = Path::new(vec![[0.0, 9222.444], [3.432, 5933.9999999],
    // [3.484, 19444.333]]); let mut path_2 = Path::new(vec![[222222.22222,
    // 5933.9999999]]); let mut path_3 = Path::new(Vec::default());
    // let ty = TypeInfo::of::<Path>();

    // path_vec.push_erased((&mut path_1 as *mut Path).cast::<u8>(), ty);
    // path_vec.push_erased((&mut path_2 as *mut Path).cast::<u8>(), ty);
    // path_vec.push_erased((&mut path_3 as *mut Path).cast::<u8>(), ty);

    // let retrieved_path_1 = path_vec.get::<Path>(0);
    // assert_eq!(retrieved_path_1.steps[0][0], 0.0);
    // assert_eq!(retrieved_path_1.steps[1][0], 3.432);

    // let retrieved_path_2 = path_vec.get::<Path>(1);
    // assert_eq!(retrieved_path_2.steps[0][0], 222222.22222);
    // assert_eq!(retrieved_path_2.steps[0][1], 5933.9999999);

    // // Check mutation works
    // let retrieved_path_1 = path_vec.get_mut::<Path>(1);
    // retrieved_path_1.steps.push([1009.0, 1500.0]);
    // assert_eq!(retrieved_path_2.steps[2][0], 1009.0);
    // assert_eq!(retrieved_path_2.steps[2][1], 1500.0);
  }

  #[test]
  fn inserting_into_works() {
    let mut health_vec = ErasedVec::new::<Health>();
    let health_1 = Health::new(100);
    let health_2 = Health::new(5483392);
    let health_3 = Health::new(25);
    let health_4 = Health::new(30);

    health_vec.push(health_1);
    health_vec.push(health_2);
    health_vec.push(health_3);

    //Insert normally and c heck the values
    health_vec.insert(1, health_4);
    assert_eq!(health_vec.get::<Health>(0).min, health_1.min);
    assert_eq!(health_vec.get::<Health>(1).min, health_4.min);
    assert_eq!(health_vec.get::<Health>(2).min, health_2.min);
    assert_eq!(health_vec.get::<Health>(3).min, health_3.min);

    let mut health_vec = ErasedVec::new::<Health>();
    let health_1 = Health::new(100);
    let health_2 = Health::new(5483392);
    let health_3 = Health::new(25);
    let mut health_4 = Health::new(30);

    health_vec.push(health_1);
    health_vec.push(health_2);
    health_vec.push(health_3);

    //Insert erased and c heck the values
    health_vec.insert_erased((&mut health_4 as *mut Health).cast::<u8>(), TypeInfo::of::<Health>(), 1);
    assert_eq!(health_vec.get::<Health>(0).min, health_1.min);
    assert_eq!(health_vec.get::<Health>(1).min, health_4.min);
    assert_eq!(health_vec.get::<Health>(2).min, health_2.min);
    assert_eq!(health_vec.get::<Health>(3).min, health_3.min);
  }

  #[test]
  fn padding_works() {
    let mut vec = ErasedVec::new::<Health>();
    vec.pad();
    vec.push(Health::new(400));
    let data = unsafe { vec.get_unchecked::<[u8; 8]>(0) };
    assert_eq!(&[0; 8], data);
    let health = vec.get::<Health>(1);
    assert_eq!(health.max, 400);
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
