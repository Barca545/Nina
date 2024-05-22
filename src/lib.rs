//Add to crate attributes?
#![feature(ptr_alignment_type)]
#![feature(unchecked_math)]
#![feature(slice_index_methods)]
#![allow(dead_code)]

// mod erased_vec;
mod errors;
mod storage;

pub mod world;

// Refactor
// -Fix the crate imports

// To Do
// -Add Component Storage / Entities
// -Add tests for using bundles with Component Storage
// -Add Resource Storage
// -Add Bundle
// -Add Query
// -Add command buffer
