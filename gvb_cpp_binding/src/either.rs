
#[repr(C)]
pub enum Either<E, T> {
  Left(E),
  Right(T),
}