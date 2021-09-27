
#[derive(Debug, Clone)]
pub struct MemoryManager {
  mem: [u8; 65536],
}

impl MemoryManager {
  pub fn set(&mut self, addr: u16, value: u8) {
    self.mem[addr as usize] = value;
  }

  pub fn get(&self, addr: u16) -> u8 {
    self.mem[addr as usize]
  }
}