use crate::{
  analyzer::Analyzer,
  build_effect,
  entity::{Entity, LiteralEntity},
  transformer::Transformer,
  utils::ast::AstKind2,
};
use oxc::ast::ast::{Expression, ImportExpression};

impl<'a> Analyzer<'a> {
  pub fn exec_import_expression(&mut self, node: &'a ImportExpression<'a>) -> Entity<'a> {
    let specifier = self.exec_expression(&node.source).get_to_string(self);
    let mut deps = self.factory.vec1(specifier);
    for option in &node.options {
      deps.push(self.exec_expression(option));
    }
    let dep = self.consumable((AstKind2::ImportExpression(node), deps));

    if let Some(LiteralEntity::String(specifier, _m)) = specifier.get_literal(self) {
      if let Some(module_id) = self.resolve_and_import_module(specifier) {
        return self.factory.computed_unknown((module_id, dep));
      }
    }

    self.factory.computed_unknown(dep)
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_import_expression(
    &self,
    node: &'a ImportExpression<'a>,
    need_val: bool,
  ) -> Option<Expression<'a>> {
    let ImportExpression { span, source, options, phase } = node;

    let need_import = need_val || self.is_referred(AstKind2::ImportExpression(node));

    let source = self.transform_expression(source, need_import);

    if need_import {
      let mut transformed_options = self.ast_builder.vec();
      for option in options {
        transformed_options.push(self.transform_expression(option, true).unwrap());
      }
      Some(self.ast_builder.expression_import(*span, source.unwrap(), transformed_options, *phase))
    } else {
      let mut effects = vec![source];
      for option in options {
        effects.push(self.transform_expression(option, false));
      }
      build_effect!(&self.ast_builder, *span, effects)
    }
  }
}
