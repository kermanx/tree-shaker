use crate::{analyzer::Analyzer, ast::AstKind2, entity::Entity, transformer::Transformer};
use oxc::{allocator::Allocator, ast::ast::JSXAttributeName, span::GetSpan};

impl<'a> Analyzer<'a> {
  pub fn exec_jsx_attribute_name(&mut self, node: &'a JSXAttributeName<'a>) -> Entity<'a> {
    self
      .exec_mangable_static_string(AstKind2::JSXAttributeName(node), get_text(self.allocator, node))
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_jsx_attribute_name_need_val(
    &self,
    node: &'a JSXAttributeName<'a>,
  ) -> JSXAttributeName<'a> {
    self.ast_builder.jsx_attribute_name_identifier(
      node.span(),
      self.transform_mangable_static_string(
        AstKind2::JSXAttributeName(node),
        get_text(self.allocator, node),
      ),
    )
  }
}

fn get_text<'a>(allocator: &'a Allocator, node: &'a JSXAttributeName<'a>) -> &'a str {
  match node {
    JSXAttributeName::Identifier(node) => node.name.as_str(),
    JSXAttributeName::NamespacedName(node) => {
      allocator.alloc_str(&format!("{}:{}", node.namespace.name, node.name))
    }
  }
}
