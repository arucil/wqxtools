use semver::Version;

pub fn is_new_version(ver: &str) -> Result<bool, semver::Error> {
  let ver = ver.parse::<Version>()?;
  let cur_ver = env!("CARGO_PKG_VERSION").parse::<Version>().unwrap();
  Ok(ver > cur_ver)
}
