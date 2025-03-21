use crate::analyzer::Analyzer;
use oxc::{
  ast::{AstKind, ast::PropertyKind},
  semantic::ScopeId,
};

impl<'a> Analyzer<'a> {
  /// Note: this is for flamegraph only. May not conform to the standard.
  pub fn resolve_function_name(&self, scope_id: ScopeId) -> Option<&'a str> {
    let node_id = self.semantic().scoping().get_node_id(scope_id);
    let parent = self.semantic().nodes().parent_kind(node_id)?;
    match parent {
      AstKind::VariableDeclarator(node) => node.id.get_identifier_name().map(|a| a.as_str()),
      AstKind::AssignmentPattern(node) => node.left.get_identifier_name().map(|a| a.as_str()),
      AstKind::AssignmentExpression(node) => node.left.get_identifier_name(),
      AstKind::ObjectProperty(node) => node.key.static_name().map(|s| {
        let kind_text = match node.kind {
          PropertyKind::Init => "",
          PropertyKind::Get => "get ",
          PropertyKind::Set => "set ",
        };
        &*self.allocator.alloc_str(&(kind_text.to_string() + &s))
      }),
      AstKind::ImportSpecifier(node) => Some(node.imported.name().as_str()),
      _ => None,
    }
  }
}
