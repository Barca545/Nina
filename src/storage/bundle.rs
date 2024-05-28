use super::type_info::TypeInfo;
use super::EcsData;
use eyre::Result;
use std::mem;

// Refactor:
// There 100% must be a better way to do the num macro

///An arbitrary tuple of [`EcsData`].
pub trait Bundle {
  ///Stores the number of items in the [`Bundle`].
  const LENGTH:usize;

  ///Takes a callback that moves components out of the bundle one-by-one.
  unsafe fn put(self, f:impl FnMut(*mut u8, TypeInfo) -> Result<()>) -> Result<()>;

  ///Returns a [`Vec`] containing the [`TypeInfo`] of all the components in the
  /// bundle.
  fn types() -> Vec<TypeInfo>;
}

macro_rules! impl_tuple {
  ($($name:ident),*) => {
    impl<$($name:EcsData),*> Bundle for ($($name,)*) {
      const LENGTH:usize = count_items!($($name),*);

      // #[allow(unused_variables, unused_mut)]
      // unsafe fn put(self, mut f: impl FnMut(*mut u8, TypeInfo) -> Result<()>) -> Result<()>{
      //   #[allow(non_snake_case)]
      //   let ($(mut $name,)*) = self;
      //   $(
      //     f(
      //       (&mut $name as *mut $name).cast::<u8>(),
      //       TypeInfo::of::<$name>(),
      //     )?;
      //     mem::forget($name);
      //   )*
      //   Ok(())
      // }
      #[allow(unused_variables, unused_mut)]
      unsafe fn put(self, mut f: impl FnMut(*mut u8, TypeInfo) -> Result<()>) -> Result<()>{
      #[allow(non_snake_case)]
      let ($($name,)*) = self;
      $(
        #[allow(non_snake_case)]
        let mut $name = mem::ManuallyDrop::new($name);
      )*
      $(
        f(
          (&mut *$name as *mut $name).cast::<u8>(),
          TypeInfo::of::<$name>(),
        )?;
      )*
      Ok(())
    }

      #[allow(unused_variables, unused_mut)]
      fn types()->Vec<TypeInfo>{
        let mut types = Vec::new();
        $(
          types.push(TypeInfo::of::<$name>());
        )*
        types
      }
    }
  };
}

macro_rules! count_items {
  () => { 0 };
  ($first:ident $(, $rest:ident)*) => { 1 + count_items!($($rest),*) };
}

macro_rules! smaller_tuples_too {
  ($m: ident, $next: tt) => {
    $m!{}
    $m!{$next}
  };
  ($m: ident, $next: tt, $($rest: tt),*) => {
    smaller_tuples_too!{$m, $($rest),*}
    reverse_apply!{$m [$next $($rest)*]}
  };
}

macro_rules! reverse_apply {
  ($m: ident [] $($reversed:tt)*) => {
    $m!{$($reversed),*}  // base case
  };
  ($m: ident [$first:tt $($rest:tt)*] $($reversed:tt)*) => {
    reverse_apply!{$m [$($rest)*] $first $($reversed)*}
  };
}

smaller_tuples_too!(impl_tuple, O, N, M, L, K, J, I, H, G, F, E, D, C, B, A);

#[cfg(test)]
mod tests {
  use crate::storage::{Bundle, TypeInfo};

  #[test]
  fn get_type_info_from_bundle() {
    let bundle_tys = <(u32, f32, String)>::types();
    assert_eq!(bundle_tys[0], TypeInfo::of::<u32>());
    assert_eq!(bundle_tys[1], TypeInfo::of::<f32>());
    assert_eq!(bundle_tys[2], TypeInfo::of::<String>());
  }

  #[test]
  fn num_in_bundle() {
    let num_0 = <(u32, f32, String)>::LENGTH;
    assert_eq!(num_0, 3);

    let num_1 = <(u32, f32, String, u32, f32, String, u32, f32, String)>::LENGTH;
    assert_eq!(num_1, 9);

    let num_1 = <(u32, f32, String, u32, f32, String, u32, f32, String, u32, f32, String, f32)>::LENGTH;
    assert_eq!(num_1, 13);
  }
}
