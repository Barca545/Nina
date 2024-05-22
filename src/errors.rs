use thiserror::Error;

#[derive(Error, Debug)]
pub enum TypeInfoErrors {
  #[error("invalid parameters to Layout::from_size_align")]
  LayoutError
}

#[derive(Debug, Error)]
pub enum ErasedVecErrors {
  #[error("This DenseVec does not contain data of type {0:?}.")]
  DoesNotContainType(String),
  #[error("Cannot insert type {insert_type:?} into vector of type {vec_type:?}.")]
  IncorrectTypeInsertion { insert_type:String, vec_type:String },
  #[error("Vector len is {len:?}. Cannot insert into {index:?}.")]
  IndexOutOfBounds { len:usize, index:usize },
  #[error("Allocation too large")]
  ErasedVecAllocError,
  #[error("Capacity overflow")]
  ErasedVecCapacityOverflow
}
