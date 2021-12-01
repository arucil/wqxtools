use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Search order:
/// - working directory
/// - executable path
pub fn load_config_file<P>(p: P) -> io::Result<String>
where
  P: AsRef<Path>,
{
  let p = p.as_ref();
  let path = if fs::try_exists(p)? {
    PathBuf::from(p)
  } else {
    env::current_exe()?.parent().unwrap().join(p)
  };
  std::fs::read_to_string(path)
}