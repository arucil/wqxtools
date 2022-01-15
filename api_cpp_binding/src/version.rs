use crate::{Maybe, Utf8Str};

#[no_mangle]
pub extern "C" fn is_new_version(ver: Utf8Str) -> Maybe<bool> {
  wqxtools::is_new_version(unsafe { ver.as_str() })
    .map_or(Maybe::Nothing, Maybe::Just)
}
