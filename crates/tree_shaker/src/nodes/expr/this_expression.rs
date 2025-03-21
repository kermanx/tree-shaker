use crate::{analyzer::Analyzer, entity::Entity, transformer::Transformer};
use oxc::ast::ast::{Expression, ThisExpression};

impl<'a> Analyzer<'a> {
  pub fn exec_this_expression(&mut self, _node: &'a ThisExpression) -> Entity<'a> {
    self.get_this()
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_this_expression(
    &self,
    node: &'a ThisExpression,
    need_val: bool,
  ) -> Option<Expression<'a>> {
    if need_val { Some(self.ast_builder.expression_this(node.span)) } else { None }
  }
}
