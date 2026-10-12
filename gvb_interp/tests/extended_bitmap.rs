use gvb_interp::device::Device;
use gvb_interp::{Document, ExecInput, ExecResult, Severity};

fn run(source: &str) -> (ExecResult, Vec<u8>, [u8; 2]) {
  gvb_interp::machine::init_machines().unwrap();
  let mut doc = Document::load(source.as_bytes(), false).unwrap();
  let errors: Vec<_> = doc
    .diagnostics()
    .iter()
    .flat_map(|line| &line.diagnostics)
    .filter(|diagnostic| diagnostic.severity == Severity::Error)
    .collect();
  assert!(errors.is_empty(), "{errors:?}");
  let device = doc.create_device(std::env::temp_dir());
  let mut vm = doc.create_vm(device).ok().unwrap();
  vm.start();
  let result = vm.exec(ExecInput::None, 10_000);
  (
    result,
    vm.device().graphic_memory().to_vec(),
    [vm.device().read_byte(4000), vm.device().read_byte(4001)],
  )
}

fn pixel(screen: &[u8], x: usize, y: usize) -> bool {
  screen[y * 20 + x / 8] & (0x80 >> (x % 8)) != 0
}

#[test]
fn load_and_paint_support_composition_and_clipping() {
  let source = "10 GRAPH\n\
20 LOAD 4000,160,85\n\
30 BOX 2,2,5,2,1,1:PAINT 4000,2,2,4,1\n\
40 BOX 2,4,5,4,1,1:PAINT 4000,2,4,4,1,3\n\
50 PAINT 4000,2,6,4,1,2\n\
60 DRAW 3,8:PAINT 4000,2,8,4,1,1\n\
70 BOX 2,10,5,10,1,1:PAINT 4000,2,10,4,1,4\n\
80 PAINT 4000,-2,12,4,1\n\
90 PAINT 4001,158,14,8,1\n\
100 PAINT 4000,20,16,8,2\n\
110 END\n";
  let (result, screen, bytes) = run(source);
  assert_eq!(result, ExecResult::End);
  assert_eq!(bytes, [160, 85]);
  for (y, expected) in [
    (2, [true, false, true, false]),
    (4, [true, false, true, false]),
    (6, [false, true, false, true]),
    (8, [true, true, true, false]),
    (10, [false, true, false, true]),
  ] {
    for (offset, on) in expected.into_iter().enumerate() {
      assert_eq!(
        pixel(&screen, 2 + offset, y),
        on,
        "row {y}, offset {offset}"
      );
    }
  }
  assert!(pixel(&screen, 0, 12));
  assert!(!pixel(&screen, 1, 12));
  assert!(!pixel(&screen, 158, 14));
  assert!(pixel(&screen, 159, 14));
  assert!(pixel(&screen, 20, 16));
  assert!(!pixel(&screen, 21, 16));
  assert!(!pixel(&screen, 20, 17));
  assert!(pixel(&screen, 21, 17));
}

#[test]
fn load_requires_byte_literals() {
  gvb_interp::machine::init_machines().unwrap();
  for source in ["10 LOAD 4000,256\n", "10 LOAD 4000,1+2\n", "10 LOAD 4000\n"] {
    let mut doc = Document::load(source.as_bytes(), false).unwrap();
    assert!(
      doc
        .diagnostics()
        .iter()
        .flat_map(|line| &line.diagnostics)
        .any(|d| d.severity == Severity::Error),
      "source accepted: {source}"
    );
  }
}

#[test]
fn load_rejects_memory_overflow() {
  let (result, _, _) = run("10 LOAD 65535,1,2\n20 END\n");
  assert!(
    matches!(result, ExecResult::Error { message, .. } if message.contains("内存"))
  );
}

#[test]
fn paint_rejects_memory_overflow() {
  let (result, _, _) = run("10 PAINT 65535,0,0,8,2\n20 END\n");
  assert!(
    matches!(result, ExecResult::Error { message, .. } if message.contains("内存"))
  );
}

#[test]
fn apostrophe_comments_skip_the_rest_of_the_line() {
  let (result, _, bytes) = run("10 LOAD 4000,42:' LOAD 4000,99\n20 END\n");
  assert_eq!(result, ExecResult::End);
  assert_eq!(bytes[0], 42);
}
