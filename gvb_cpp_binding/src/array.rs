#[repr(C)]
pub struct Array<T> {
  pub data: *const T,
  pub len: usize,
}

impl<T> Array<T> {
  pub(crate) unsafe fn new(v: Vec<T>) -> Self {
    let len = v.len();
    let data =
      Box::into_raw(Box::<[_]>::from(v.into_boxed_slice())).as_mut_ptr();
    Self { data, len }
  }
}
