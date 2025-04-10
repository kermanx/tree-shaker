use oxc::{
  ast::ast::{CatchClause, CatchParameter},
  span::GetSpan,
};

use crate::{analyzer::Analyzer, ast::DeclarationKind, entity::Entity, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn exec_catch_clause(&mut self, node: &'a CatchClause<'a>, value: Entity<'a>) {
    self.push_indeterminate_cf_scope();
    self.set_variable_scope_depth(node.scope_id());

    if let Some(param) = &node.param {
      self.declare_binding_pattern(&param.pattern, false, DeclarationKind::Caught);
      self.init_binding_pattern(&param.pattern, Some(value));
    }

    self.exec_block_statement(&node.body);

    self.pop_cf_scope();
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_catch_clause(&self, node: &'a CatchClause<'a>) -> CatchClause<'a> {
    let CatchClause { span, param, body, .. } = node;

    let param = param.as_ref().and_then(|param| {
      let CatchParameter { span, pattern } = param;
      self
        .transform_binding_pattern(pattern, false)
        .map(|pattern| self.ast_builder.catch_parameter(*span, pattern))
    });

    let body_span = body.span();
    let body = self.transform_block_statement(body);

    self.ast_builder.catch_clause(
      *span,
      param,
      body.unwrap_or(self.ast_builder.alloc_block_statement(body_span, self.ast_builder.vec())),
    )
  }
}
