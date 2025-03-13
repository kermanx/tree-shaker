use crate::{analyzer::Analyzer, ast::AstKind2, entity::Entity, transformer::Transformer};
use oxc::{
  ast::{
    ast::{ChainElement, Expression, MemberExpression},
    match_member_expression,
  },
  span::{GetSpan, SPAN},
};

/// Returns Some((node, same_chain))
fn unwrap_to_member_expression<'a>(
  node: &'a Expression<'a>,
) -> Option<(&'a MemberExpression<'a>, bool)> {
  match node {
    match_member_expression!(Expression) => Some((node.to_member_expression(), true)),
    Expression::ParenthesizedExpression(node) => {
      unwrap_to_member_expression(&node.expression).map(|(node, _)| (node, false))
    }
    Expression::ChainExpression(node) => match &node.expression {
      match_member_expression!(ChainElement) => {
        Some((node.expression.to_member_expression(), false))
      }
      _ => None,
    },
    _ => None,
  }
}

impl<'a> Analyzer<'a> {
  /// Returns: Ok((scope_count, callee, undefined, this)) or Err(forwarded_undefined) for should not call due to ?. operator
  pub fn exec_callee(
    &mut self,
    node: &'a Expression<'a>,
  ) -> Result<(usize, Entity<'a>, Option<Entity<'a>>, Entity<'a>), Entity<'a>> {
    if matches!(node, Expression::Super(_)) {
      return Ok((0, self.get_super(), None, self.get_this()));
    }

    let dep = AstKind2::Callee(node);
    if let Some((member_expr, same_chain)) = unwrap_to_member_expression(node) {
      if same_chain {
        let (scope_count, callee, undefined, (object, _)) =
          self.exec_member_expression_read_in_chain(member_expr, false)?;
        Ok((scope_count, callee, undefined, self.factory.computed(object, dep)))
      } else {
        let result = self.exec_member_expression_read_in_chain(member_expr, false);
        Ok(match result {
          Ok((scope_count, value, undefined, (object, _))) => {
            self.pop_multiple_cf_scopes(scope_count);
            (
              0,
              self.factory.optional_union(value, undefined),
              None,
              self.factory.computed(object, dep),
            )
          }
          Err(value) => (0, value, None, self.factory.immutable_unknown),
        })
      }
    } else {
      let (scope_count, callee, undefined) = self.exec_expression_in_chain(node)?;
      Ok((scope_count, callee, undefined, self.factory.computed(self.factory.undefined, dep)))
    }
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_callee(
    &self,
    node: &'a Expression<'a>,
    need_val: bool,
  ) -> Result<Option<Expression<'a>>, Option<Expression<'a>>> {
    if need_val {
      let transformed_expr = self.transform_expression_in_chain(node, true)?.unwrap();

      let is_referred = self.is_referred(AstKind2::Callee(node));
      let was_member_expression = unwrap_to_member_expression(node).is_some();
      let is_member_expression = unwrap_to_member_expression(&transformed_expr).is_some();
      Ok(Some(if is_referred && !was_member_expression && is_member_expression {
        self.ast_builder.expression_sequence(transformed_expr.span(), {
          let mut seq = self.ast_builder.vec_with_capacity(2);
          seq.push(self.build_unused_expression(SPAN));
          seq.push(transformed_expr);
          seq
        })
      } else {
        transformed_expr
      }))
    } else {
      self.transform_expression_in_chain(node, false)
    }
  }
}
