use crate::errors::TypeInfoErrors;
use eyre::Result;
use std::{alloc::Layout, any::TypeId, hash::Hash};

// Refactor:
// -Could replace the array implementation
// Layout::from_size_align_unchecked(
//   info.layout.size() * self.entities.len(),
//   info.layout.align(),
// ),

#[derive(Debug, Copy, Clone)]
/// Metadata required to store a component.
///
/// All told, this means a [`TypeId`], to be able to dynamically name/check the
/// component type; a [`Layout`], so that we know how to allocate memory for
/// this component type; and a drop function which internally calls
/// [`core::ptr::drop_in_place`] with the correct type parameter.
pub struct TypeInfo {
  id:TypeId,
  layout:Layout,
  drop:unsafe fn(*mut u8),
  type_name:&'static str
}

impl TypeInfo {
  pub fn of<T:'static>() -> Self {
    unsafe fn drop_ptr<T>(x:*mut u8) {
      x.cast::<T>().drop_in_place()
    }

    Self {
      id:TypeId::of::<T>(),
      layout:Layout::new::<T>(),
      drop:drop_ptr::<T>,
      #[cfg(debug_assertions)]
      type_name:core::any::type_name::<T>()
    }
  }

  /// Access the [`TypeId`] for this component type.
  pub fn id(&self) -> TypeId {
    self.id
  }

  /// Access the [`Layout`] of this component type.
  pub fn layout(&self) -> Layout {
    self.layout
  }

  /// Access the name of this component type.
  pub fn name(&self) -> String {
    self.type_name.to_string()
  }

  /// Access the size of this component type.
  pub fn size(&self) -> usize {
    self.layout.size()
  }

  ///Creates a [`Layout`] describing the record for a [T; n] where T is the
  /// type described by a [`TypeInfo`].
  ///
  ///On arithmetic overflow or when the total size would exceed isize::MAX,
  /// returns LayoutError.
  ///
  ///Type-erased implementation of [`Layout`]'s [array method](https://doc.rust-lang.org/src/core/alloc/layout.rs.html#433).
  pub fn array(&self, n:usize) -> Result<Layout, TypeInfoErrors> {
    let element_size = self.layout().size();
    let align = self.layout().align();
    return inner(element_size, align, n);

    #[inline]
    const fn inner(element_size:usize, align:usize, n:usize) -> Result<Layout, TypeInfoErrors> {
      // We need to check two things about the size:
      //  - That the total size won't overflow a `usize`, and
      //  - That the total size still fits in an `isize`.
      // By using division we can check them both with a single threshold.
      // That'd usually be a bad idea, but thankfully here the element size
      // and alignment are constants, so the compiler will fold all of it.
      if element_size != 0 && n > (isize::MAX as usize - (align - 1)) / element_size {
        return Err(TypeInfoErrors::LayoutError);
      }
      let array_size = unsafe { element_size.unchecked_mul(n) };

      unsafe { Ok(Layout::from_size_align_unchecked(array_size, align)) }
    }
  }

  /// Directly call the destructor on a pointer to data of this component type.
  ///
  /// # Safety
  ///
  /// All of the caveats of [`core::ptr::drop_in_place`] apply, with the
  /// additional requirement that this method is being called on a pointer to
  /// an object of the correct component type.
  pub unsafe fn drop(&self, data:*mut u8) {
    (self.drop)(data)
  }

  /// Get the function pointer encoding the destructor for the component type
  /// this [`TypeInfo`] represents.
  pub fn drop_shim(&self) -> unsafe fn(*mut u8) {
    self.drop
  }
}

impl PartialOrd for TypeInfo {
  fn partial_cmp(&self, other:&Self) -> Option<core::cmp::Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for TypeInfo {
  /// Order by alignment, descending. Ties broken with TypeId.
  fn cmp(&self, other:&Self) -> core::cmp::Ordering {
    self
      .layout
      .align()
      .cmp(&other.layout.align())
      .reverse()
      .then_with(|| self.id.cmp(&other.id))
  }
}

impl PartialEq for TypeInfo {
  fn eq(&self, other:&Self) -> bool {
    self.id == other.id
  }
}

impl Hash for TypeInfo {
  fn hash<H:std::hash::Hasher>(&self, state:&mut H) {
    self.id.hash(state);
  }
}

impl Eq for TypeInfo {}
