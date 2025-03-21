use crate::{analyzer::Analyzer, entity::Entity, transformer::Transformer};
use oxc::{
  allocator,
  ast::ast::{Expression, JSXChild},
};

impl<'a> Analyzer<'a> {
  pub fn exec_jsx_children(&mut self, node: &'a allocator::Vec<'a, JSXChild<'a>>) -> Entity<'a> {
    let mut values = self.factory.vec();
    for child in node {
      values.push(match child {
        JSXChild::Text(node) => self.exec_jsx_text(node),
        JSXChild::Element(node) => self.exec_jsx_element(node),
        JSXChild::Fragment(node) => self.exec_jsx_fragment(node),
        JSXChild::ExpressionContainer(node) => {
          self.exec_jsx_expression_container_as_jsx_child(node)
        }
        JSXChild::Spread(node) => self.exec_jsx_spread_child(node),
      });
    }
    self.factory.computed_unknown(values)
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_jsx_children_effect_only(
    &self,
    node: &'a allocator::Vec<'a, JSXChild<'a>>,
  ) -> Vec<Expression<'a>> {
    node
      .iter()
      .filter_map(|child| match child {
        JSXChild::Text(node) => self.transform_jsx_text_effect_only(node),
        JSXChild::Element(node) => self.transform_jsx_element_effect_only(node),
        JSXChild::Fragment(node) => self.transform_jsx_fragment_effect_only(node),
        JSXChild::ExpressionContainer(node) => {
          self.transform_jsx_expression_container_effect_only(node)
        }
        JSXChild::Spread(node) => self.transform_jsx_spread_child_effect_only(node),
      })
      .collect()
  }

  pub fn transform_jsx_children_need_val(
    &self,
    node: &'a allocator::Vec<'a, JSXChild<'a>>,
  ) -> allocator::Vec<'a, JSXChild<'a>> {
    let mut transformed = self.ast_builder.vec_with_capacity(node.len());

    for child in node.iter() {
      transformed.push(match child {
        JSXChild::Text(node) => self.transform_jsx_text_need_val(node),
        JSXChild::Element(node) => JSXChild::Element(self.transform_jsx_element_need_val(node)),
        JSXChild::Fragment(node) => JSXChild::Fragment(self.transform_jsx_fragment_need_val(node)),
        JSXChild::ExpressionContainer(node) => {
          JSXChild::ExpressionContainer(self.transform_jsx_expression_container_need_val(node))
        }
        JSXChild::Spread(node) => JSXChild::Spread(self.transform_jsx_spread_child_need_val(node)),
      })
    }

    transformed
  }
}
