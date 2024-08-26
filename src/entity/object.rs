use super::{
  entity::{Entity, EntityTrait},
  literal::LiteralEntity,
  unknown::UnknownEntity,
};
use crate::analyzer::Analyzer;
use rustc_hash::FxHashMap;

#[derive(Debug, Default)]
pub(crate) struct ObjectEntity<'a> {
  string_keyed: FxHashMap<&'a str, Entity<'a>>,
  // TODO: symbol_keyed
  rest: Option<Entity<'a>>,
}

impl<'a> EntityTrait<'a> for ObjectEntity<'a> {
  fn consume_self(&self, analyzer: &mut Analyzer<'a>) {}

  fn consume_as_unknown(&self, analyzer: &mut Analyzer<'a>) {
    for (_, value) in &self.string_keyed {
      value.consume_self(analyzer);
    }
    if let Some(rest) = &self.rest {
      rest.consume_self(analyzer);
    }
  }

  fn get_property(&self, key: &Entity<'a>) -> Entity<'a> {
    // FIXME: p4 rest
    match key.get_literal() {
      Some(LiteralEntity::String(key)) => {
        if let Some(value) = self.string_keyed.get(key) {
          value.clone()
        } else {
          UnknownEntity::new_unknown()
        }
      }
      _ => UnknownEntity::new_unknown(),
    }
  }

  fn get_literal(&self) -> Option<LiteralEntity<'a>> {
    None
  }

  fn test_truthy(&self) -> Option<bool> {
    Some(true)
  }

  fn test_nullish(&self) -> Option<bool> {
    Some(false)
  }

  fn test_is_undefined(&self) -> Option<bool> {
    Some(false)
  }
}

impl<'a> ObjectEntity<'a> {
  pub(crate) fn set_property(&mut self, key: Entity<'a>, value: Entity<'a>) {
    match key.get_literal() {
      Some(LiteralEntity::String(key)) => {
        self.string_keyed.insert(key, value);
      }
      _ => todo!("p4"),
    }
  }

  pub(crate) fn set_rest(&mut self, rest: Entity<'a>) {
    self.rest = Some(rest);
  }
}
