use crate::{Array, Either, Maybe, Rect, Unit, Utf8Str, Utf8String};
use gvb_interp as gvb;
use gvb_interp::machine::{self, InitError};

pub type GvbInitMachineResult = Either<Utf8String, Unit>;

pub struct GvbDevice(pub(crate) gvb::device::default::DefaultDevice);

#[no_mangle]
pub extern "C" fn gvb_init_machines() -> GvbInitMachineResult {
  match machine::init_machines() {
    Ok(()) => Either::Right(Unit::new()),
    Err(err) => match err {
      InitError::Io(err) => Either::Left(unsafe {
        Utf8String::new(format!("读取机型配置文件失败：{}", err))
      }),
      InitError::Yaml(err) => Either::Left(unsafe {
        Utf8String::new(format!("解析机型配置文件失败：{}", err))
      }),
      InitError::Other(err) => Either::Left(unsafe {
        Utf8String::new(format!("机型配置文件错误：{}", err))
      }),
    },
  }
}

#[no_mangle]
pub extern "C" fn gvb_machine_names() -> Array<Utf8Str> {
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
pub extern "C" fn gvb_destroy_device(dev: *mut GvbDevice) {
  drop(unsafe { Box::from_raw(dev) });
}

#[no_mangle]
pub extern "C" fn gvb_device_graphics_memory(dev: *mut GvbDevice) -> *const u8 {
  unsafe { (*dev).0.graphic_memory().as_ptr() }
}

#[no_mangle]
pub extern "C" fn gvb_device_reset(dev: *mut GvbDevice) {
  unsafe {
    (*dev).0.reset();
  }
}

#[no_mangle]
pub extern "C" fn gvb_device_fire_key_down(dev: *mut GvbDevice, key: u8) {
  unsafe {
    (*dev).0.fire_key_down(key);
  }
}

#[no_mangle]
pub extern "C" fn gvb_device_fire_key_up(dev: *mut GvbDevice, key: u8) {
  unsafe {
    (*dev).0.fire_key_up(key);
  }
}

#[no_mangle]
pub extern "C" fn gvb_device_blink_cursor(dev: *mut GvbDevice) {
  unsafe {
    (*dev).0.blink_cursor();
  }
}

#[no_mangle]
pub extern "C" fn gvb_device_screen_dirty_area(
  dev: *mut GvbDevice,
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
