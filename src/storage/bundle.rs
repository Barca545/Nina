use super::type_info::TypeInfo;
use super::EcsData;
use eyre::Result;
use std::mem;

pub trait Bundle {
  ///Takes a callback that moves components out of the bundle one-by-one.
  unsafe fn put(self, f:impl FnMut(*mut u8, TypeInfo) -> Result<()>) -> Result<()>;

  ///Returns a [`Vec`] containing the [`TypeInfo`] of all the components in the
  /// bundle.
  fn types() -> Vec<TypeInfo>;
}

macro_rules! impl_tuple {
  ($($name:ident),*) => {
    impl<$($name:EcsData),*> Bundle for ($($name,)*) {
      #[allow(unused_variables, unused_mut)]
      unsafe fn put(self, mut f: impl FnMut(*mut u8, TypeInfo) -> Result<()>) -> Result<()>{
        #[allow(non_snake_case)]
        let ($(mut $name,)*) = self;
        $(
          f(
            (&mut $name as *mut $name).cast::<u8>(),
            TypeInfo::of::<$name>(),
          )?;
          mem::forget($name);
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
