use id_arena::{Id, Arena};

pub struct Rule(Id);

struct RuleInner {
  name: String,
  items: Vec<Item>
}

pub struct RuleBuilder(Inner);

struct Inner {
  arena: Arena<RuleInner>
}

impl RuleBuilder {
  pub fn new() -> Self {
    Self(Inner {
      arena: Arena::new()
    })
  }

  pub fn rule(&mut self, name: &str, items: &[Item]) -> Rule {
  }
}

pub struct Item;

impl Item {
}