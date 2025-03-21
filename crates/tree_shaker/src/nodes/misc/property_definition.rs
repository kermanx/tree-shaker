use crate::{transformer::Transformer, utils::ast::AstKind2};
use oxc::ast::{
  NONE,
  ast::{ClassElement, PropertyDefinition},
};

impl<'a> Transformer<'a> {
  pub fn transform_property_definition(
    &self,
    node: &'a PropertyDefinition<'a>,
  ) -> Option<ClassElement<'a>> {
    let PropertyDefinition { r#type, span, decorators, key, value, computed, r#static, .. } = node;

    if !self.is_referred(AstKind2::PropertyDefinition(node)) {
      return None;
    }

    let key = self.transform_property_key(key, true).unwrap();
    let value = value.as_ref().map(|node| self.transform_expression(node, true).unwrap());

    Some(self.ast_builder.class_element_property_definition(
      *span,
      *r#type,
      self.clone_node(decorators),
      key,
      value,
      *computed,
      *r#static,
      false,
      false,
      false,
      false,
      false,
      NONE,
      None,
    ))
  }
}
