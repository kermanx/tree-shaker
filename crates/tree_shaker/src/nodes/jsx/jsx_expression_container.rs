use crate::{
  analyzer::Analyzer,
  ast::AstKind2,
  build_effect,
  entity::{Entity, LiteralEntity},
  transformer::Transformer,
};
use oxc::{
  allocator,
  ast::ast::{Expression, JSXExpression, JSXExpressionContainer},
  span::GetSpan,
};

impl<'a> Analyzer<'a> {
  pub fn exec_jsx_expression_container_as_attribute_value(
    &mut self,
    node: &'a JSXExpressionContainer<'a>,
  ) -> Entity<'a> {
    match &node.expression {
      JSXExpression::EmptyExpression(_node) => self.factory.r#true,
      node => self.exec_expression(node.to_expression()),
    }
  }

  pub fn exec_jsx_expression_container_as_jsx_child(
    &mut self,
    node: &'a JSXExpressionContainer<'a>,
  ) -> Entity<'a> {
    let value = match &node.expression {
      JSXExpression::EmptyExpression(_node) => self.factory.string(""),
      node => self.exec_expression(node.to_expression()).get_to_jsx_child(self),
    };
    self.try_fold_node(AstKind2::JsxExpressionContainer(node), value)
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_jsx_expression_container_effect_only(
    &self,
    node: &'a JSXExpressionContainer<'a>,
  ) -> Option<Expression<'a>> {
    match &node.expression {
      JSXExpression::EmptyExpression(_node) => None,
      node => self.transform_expression(node.to_expression(), false),
    }
  }

  pub fn transform_jsx_expression_container_need_val(
    &self,
    node: &'a JSXExpressionContainer<'a>,
  ) -> allocator::Box<'a, JSXExpressionContainer<'a>> {
    let JSXExpressionContainer { span, expression } = node;

    self.ast_builder.alloc_jsx_expression_container(
      *span,
      if let Some(literal) = self.build_folded_expr(AstKind2::JsxExpressionContainer(node)) {
        let effect = self.transform_jsx_expression_container_effect_only(node);
        if effect.is_none()
          && matches!(
            self.get_folded_literal(AstKind2::JsxExpressionContainer(node)).unwrap(),
            LiteralEntity::String("", _)
          )
        {
          self.ast_builder.jsx_expression_empty_expression(expression.span())
        } else {
          JSXExpression::from(build_effect!(self.ast_builder, expression.span(), effect; literal))
        }
      } else {
        match expression {
          JSXExpression::EmptyExpression(node) => {
            self.ast_builder.jsx_expression_empty_expression(node.span)
          }
          node => {
            JSXExpression::from(self.transform_expression(node.to_expression(), true).unwrap())
          }
        }
      },
    )
  }
}
