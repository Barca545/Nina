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

// Refactor:
// -Should CreateComponentNeverCalled and ComponentNotRegistered be different?

#[derive(Debug, Error)]
pub enum EcsErrors {
  #[error("Attempting to add {component:?} to an entitity without registering it first!")]
  CreateComponentNeverCalled { component:String },
  #[error("Attempted to use an unregisted component")]
  ComponentNotRegistered,
  #[error("Attempted to reference an entity that does not exist")]
  EntityDoesNotExist,
  #[error("Attempted to access {component:?} which does not exist")]
  ResourceDataDoesNotExist { component:String },
  #[error("Attempted to use component data that does not exist")]
  ComponentDataDoesNotExist,
  #[error("Attempted to downcast component to the wrong type")]
  DowncastToWrongType,
  #[error("No resource found at given path")]
  NoResourceAtPath,
  #[error("Unable to read the exe at the given path")]
  ExeResourceRegistrationFailed
}
