#[repr(C)]
pub enum Either<E, T> {
  Left(E),
  Right(T),
}

#[repr(C)]
pub enum Maybe<T> {
  Just(T),
  Nothing,
}

#[repr(C)]
pub struct Rect {
  pub left: usize,
  pub top: usize,
  pub right: usize,
  pub bottom: usize,
}