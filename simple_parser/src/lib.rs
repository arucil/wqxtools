
struct ParserBuilder;

impl ParserBuilder {
  fn rule(&mut self, name: &str, symbols: Vec<Item>) -> Rule {
    Rule
  }
}

enum Item {
  PendingRuleRef(String),
  ResolvedRuleRef(Rule),
  Token(TokenKind),
  Alt(Vec<Item>, Vec<Item>)
}

fn rule_ref(r#ref: impl RuleRef) -> Item {
  r#ref.item()
}

fn token(kind: TokenKind) -> Item {
  Item::Token(kind)
}

fn alt(items1: Vec<Item>, items2: Vec<Item>) -> Item {
  Item::Alt(items1, items2)
}

trait RuleRef {
  fn item(self) -> Item;
}

impl<'a> RuleRef for &'a str {
  fn item(self) -> Item {
    Item::PendingRuleRef(self.to_owned())
  }
}

impl RuleRef for Rule {
  fn item(self) -> Item {
    Item::ResolvedRuleRef(self)
  }
}

#[derive(Clone, Copy)]
struct Rule;

enum TokenKind {
  Add,
  Sub,
  Mul,
  Div,
  LParen,
  RParen,
  Int,
}

fn test() {
  let mut builder = ParserBuilder;

  let expr = builder.rule("expr",
    vec![
      rule_ref("factor"),
      alt(
        vec![ token(TokenKind::Add) ],
        vec![ token(TokenKind::Sub) ],
      )
    ]);

  ()
}