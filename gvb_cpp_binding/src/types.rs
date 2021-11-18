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
