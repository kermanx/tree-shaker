use crate::{entity::Entity, Analyzer};
use oxc::ast::ast::{BooleanLiteral, NumericLiteral, StringLiteral};

#[derive(Debug, Default, Clone)]
pub struct Data {}

impl<'a> Analyzer<'a> {
  pub(crate) fn exc_numeric_literal(&mut self, node: &'a NumericLiteral) -> Entity {
    Entity::NumberLiteral(node.value)
  }

  pub(crate) fn exec_string_literal(&mut self, node: &'a StringLiteral) -> Entity {
    Entity::StringLiteral(node.value.to_string())
  }

  pub(crate) fn exec_boolean_literal(&mut self, node: &'a BooleanLiteral) -> Entity {
    Entity::BooleanLiteral(node.value)
  }
}
