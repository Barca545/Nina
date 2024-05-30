//!  ```
//! #  use nina::world::World;
//!
//!  let mut world = World::new();
//!
//!  world
//!    .register_component::<u32>()
//!    .register_component::<i32>()
//!    .register_component::<f32>();
//!
//!  world
//!    .create_entity()
//!    .with_component(4)
//!    .unwrap()
//!    .with_component(-5)
//!    .unwrap()
//!    .with_component(100.0_f32)
//!    .unwrap();
//!  ```

//Add to crate attributes?
#![feature(ptr_alignment_type)]
#![feature(unchecked_math)]
#![feature(slice_index_methods)]
#![allow(dead_code)]

mod errors;
pub mod storage;
pub mod world;

// Refactor
// -Fix the crate imports
// -I think it's worth moving Arena into storage
// -Update the documentation example
