use crate::{analyzer::Analyzer, entity::Entity, transformer::Transformer};
use oxc::ast::ast::{Expression, JSXChild, JSXText};

impl<'a> Analyzer<'a> {
  pub fn exec_jsx_text(&mut self, _node: &'a JSXText<'a>) -> Entity<'a> {
    self.factory.immutable_unknown
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_jsx_text_effect_only(&self, _node: &'a JSXText<'a>) -> Option<Expression<'a>> {
    None
  }

  pub fn transform_jsx_text_need_val(&self, node: &'a JSXText<'a>) -> JSXChild<'a> {
    let JSXText { span, value, raw } = node;

    self.ast_builder.jsx_child_text(*span, value, *raw)
  }
}
