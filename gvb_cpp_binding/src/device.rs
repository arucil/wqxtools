use crate::{Array, Either, Unit, Utf8Str, Utf8String, Maybe};
use gvb_interp as gvb;
use gvb_interp::machine::{self, InitError};

type InitMachineResult = Either<Utf8String, Unit>;

pub struct Device(pub(crate) gvb::device::default::DefaultDevice);

#[repr(C)]
pub struct Rect {
  pub left: usize,
  pub top: usize,
  pub right: usize,
  pub bottom: usize,
}

#[no_mangle]
pub extern "C" fn init_machines() -> InitMachineResult {
  match machine::init_machines() {
    Ok(()) => Either::Right(Unit::new()),
    Err(err) => match err {
      InitError::Io(err) => Either::Left(unsafe {
        Utf8String::new(format!("读取配置文件失败：{}", err))
      }),
      InitError::Yaml(err) => Either::Left(unsafe {
        Utf8String::new(format!("解析配置文件失败：{}", err))
      }),
      InitError::Other(err) => Either::Left(unsafe {
        Utf8String::new(format!("配置文件错误：{}", err))
      }),
    },
  }
}

#[no_mangle]
pub extern "C" fn machine_names() -> Array<Utf8Str> {
  unsafe {
    Array::new(
      machine::names()
        .into_iter()
        .map(|s| Utf8Str::new(s))
        .collect(),
    )
  }
}

#[no_mangle]
pub extern "C" fn destroy_device(dev: *mut Device) {
  drop(unsafe { Box::from_raw(dev) });
}

#[no_mangle]
pub extern "C" fn device_graphics_memory(dev: *mut Device) -> *const u8 {
  unsafe { (*dev).0.graphic_memory().as_ptr() }
}

#[no_mangle]
pub extern "C" fn device_key(dev: *mut Device) -> Maybe<u8> {
  match unsafe { (*dev).0.key() } {
    Some(key) => Maybe::Just(key),
    None => Maybe::Nothing,
  }
}

#[no_mangle]
pub extern "C" fn device_fire_key_down(dev: *mut Device, key: u8) {
  unsafe {
    (*dev).0.fire_key_down(key);
  }
}

#[no_mangle]
pub extern "C" fn device_fire_key_up(dev: *mut Device, key: u8) {
  unsafe {
    (*dev).0.fire_key_up(key);
  }
}

#[no_mangle]
pub extern "C" fn device_blink_cursor(dev: *mut Device) {
  unsafe {
    (*dev).0.blink_cursor();
  }
}

#[no_mangle]
pub extern "C" fn device_screen_dirty_area(
  dev: *mut Device,
) -> Maybe<Rect> {
  unsafe {
    match (*dev).0.take_dirty_area() {
      Some(rect) => Maybe::Just(Rect {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
      }),
      None => Maybe::Nothing,
    }
  }
}
