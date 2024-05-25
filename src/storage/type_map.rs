use super::type_info::TypeInfo;
use hashbrown::{hash_map::DefaultHashBuilder, HashMap};
use std::hash::{BuildHasher, BuildHasherDefault, Hasher};

#[derive(Default)]
pub struct TypeIdHasher {
  hash:u64
}

impl Hasher for TypeIdHasher {
  fn write_u64(&mut self, n:u64) {
    // Only a single value can be hashed, so the old hash should be zero.
    debug_assert_eq!(self.hash, 0);
    self.hash = n;
  }

  // Tolerate TypeId being either u64 or u128.
  fn write_u128(&mut self, n:u128) {
    debug_assert_eq!(self.hash, 0);
    self.hash = n as u64;
  }

  fn write(&mut self, bytes:&[u8]) {
    debug_assert_eq!(self.hash, 0);

    // This will only be called if TypeId is neither u64 nor u128, which is not
    // anticipated. In that case we'll just fall back to using a different hash
    // implementation.
    let mut hasher = <DefaultHashBuilder as BuildHasher>::Hasher::default();
    hasher.write(bytes);
    self.hash = hasher.finish();
  }

  fn finish(&self) -> u64 {
    self.hash
  }
}

pub type TypeMap<V> = HashMap<TypeInfo, V, BuildHasherDefault<TypeIdHasher>>;
