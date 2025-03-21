use crate::{
  analyzer::Analyzer,
  ast::{AstKind2, DeclarationKind},
  consumable::Consumable,
  entity::Entity,
  scope::VariableScopeId,
  transformer::Transformer,
  utils::{CalleeInfo, CalleeNode},
};
use oxc::ast::{
  NONE,
  ast::{ArrowFunctionExpression, Expression},
};

impl<'a> Analyzer<'a> {
  pub fn exec_arrow_function_expression(
    &mut self,
    node: &'a ArrowFunctionExpression<'a>,
  ) -> Entity<'a> {
    self.new_function(CalleeNode::ArrowFunctionExpression(node)).into()
  }

  pub fn call_arrow_function_expression(
    &mut self,
    callee: CalleeInfo<'a>,
    call_dep: Consumable<'a>,
    node: &'a ArrowFunctionExpression<'a>,
    variable_scopes: &'a [VariableScopeId],
    args: Entity<'a>,
    consume: bool,
  ) -> Entity<'a> {
    self.push_call_scope(callee, call_dep, variable_scopes.to_vec(), node.r#async, false, consume);

    self.exec_formal_parameters(&node.params, args, DeclarationKind::ArrowFunctionParameter);
    if node.expression {
      self.exec_function_expression_body(&node.body);
    } else {
      self.exec_function_body(&node.body);
    }

    if consume {
      self.consume_return_values();
    }

    self.pop_call_scope()
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_arrow_function_expression(
    &self,
    node: &'a ArrowFunctionExpression<'a>,
    need_val: bool,
  ) -> Option<Expression<'a>> {
    if need_val || self.is_referred(AstKind2::ArrowFunctionExpression(node)) {
      let ArrowFunctionExpression { span, expression, r#async, params, body, .. } = node;

      let params = self.transform_formal_parameters(params);
      let body = if *expression {
        self.transform_function_expression_body(body)
      } else {
        self.transform_function_body(node.scope_id.get().unwrap(), body)
      };

      Some(self.ast_builder.expression_arrow_function(
        *span,
        *expression,
        *r#async,
        NONE,
        params,
        NONE,
        body,
      ))
    } else {
      None
    }
  }
}
