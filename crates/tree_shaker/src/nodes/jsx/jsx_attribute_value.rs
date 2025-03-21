use crate::{analyzer::Analyzer, entity::Entity, transformer::Transformer};
use oxc::{
  ast::ast::{Expression, JSXAttributeValue},
  span::Span,
};

impl<'a> Analyzer<'a> {
  pub fn exec_jsx_attribute_value(
    &mut self,
    node: &'a Option<JSXAttributeValue<'a>>,
  ) -> Entity<'a> {
    if let Some(node) = node {
      match node {
        JSXAttributeValue::StringLiteral(node) => self.exec_string_literal(node),
        JSXAttributeValue::ExpressionContainer(node) => {
          self.exec_jsx_expression_container_as_attribute_value(node)
        }
        JSXAttributeValue::Element(node) => self.exec_jsx_element(node),
        JSXAttributeValue::Fragment(node) => self.exec_jsx_fragment(node),
      }
    } else {
      self.factory.r#true
    }
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_jsx_attribute_value_effect_only(
    &self,
    node: &'a Option<JSXAttributeValue<'a>>,
  ) -> Option<Expression<'a>> {
    if let Some(node) = node {
      match node {
        JSXAttributeValue::StringLiteral(_node) => None,
        JSXAttributeValue::ExpressionContainer(node) => {
          self.transform_jsx_expression_container_effect_only(node)
        }
        JSXAttributeValue::Element(node) => self.transform_jsx_element_effect_only(node),
        JSXAttributeValue::Fragment(node) => self.transform_jsx_fragment_effect_only(node),
      }
    } else {
      None
    }
  }

  pub fn transform_jsx_attribute_value_as_item(
    &self,
    node: &'a Option<JSXAttributeValue<'a>>,
    need_val: bool,
  ) -> Option<JSXAttributeValue<'a>> {
    node.as_ref().and_then(|node| match node {
      JSXAttributeValue::StringLiteral(node) => {
        need_val.then(|| JSXAttributeValue::StringLiteral(self.clone_node(node)))
      }
      JSXAttributeValue::ExpressionContainer(node) => {
        if need_val {
          Some(JSXAttributeValue::ExpressionContainer(
            self.transform_jsx_expression_container_need_val(node),
          ))
        } else {
          self
            .transform_jsx_expression_container_effect_only(node)
            .map(|effect| self.build_jsx_expression_container_from_expression(node.span, effect))
        }
      }
      JSXAttributeValue::Element(node) => {
        if need_val {
          Some(JSXAttributeValue::Element(self.transform_jsx_element_need_val(node)))
        } else {
          self
            .transform_jsx_element_effect_only(node)
            .map(|effect| self.build_jsx_expression_container_from_expression(node.span, effect))
        }
      }
      JSXAttributeValue::Fragment(node) => {
        if need_val {
          Some(JSXAttributeValue::Fragment(self.transform_jsx_fragment_need_val(node)))
        } else {
          self
            .transform_jsx_fragment_effect_only(node)
            .map(|effect| self.build_jsx_expression_container_from_expression(node.span, effect))
        }
      }
    })
  }

  fn build_jsx_expression_container_from_expression(
    &self,
    span: Span,
    expression: Expression<'a>,
  ) -> JSXAttributeValue<'a> {
    self.ast_builder.jsx_attribute_value_expression_container(span, expression.into())
  }
}
