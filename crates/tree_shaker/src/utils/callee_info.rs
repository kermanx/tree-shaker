use super::ast::AstKind2;
use crate::{analyzer::Analyzer, module::ModuleId};
use oxc::{
  ast::ast::{ArrowFunctionExpression, Class, Function},
  span::{GetSpan, Span},
};
use std::hash;

#[derive(Debug, Clone, Copy)]
pub enum CalleeNode<'a> {
  Function(&'a Function<'a>),
  ArrowFunctionExpression(&'a ArrowFunctionExpression<'a>),
  ClassStatics(&'a Class<'a>),
  ClassConstructor(&'a Class<'a>),
  Root,
  Module,
}

impl<'a> From<CalleeNode<'a>> for AstKind2<'a> {
  fn from(val: CalleeNode<'a>) -> Self {
    match val {
      CalleeNode::Function(node) => AstKind2::Function(node),
      CalleeNode::ArrowFunctionExpression(node) => AstKind2::ArrowFunctionExpression(node),
      CalleeNode::ClassStatics(node) => AstKind2::Class(node),
      CalleeNode::ClassConstructor(node) => AstKind2::ClassConstructor(node),
      CalleeNode::Root | CalleeNode::Module => AstKind2::Environment,
    }
  }
}

impl GetSpan for CalleeNode<'_> {
  fn span(&self) -> Span {
    match self {
      CalleeNode::Function(node) => node.span(),
      CalleeNode::ArrowFunctionExpression(node) => node.span(),
      CalleeNode::ClassStatics(node) => node.span(),
      CalleeNode::ClassConstructor(node) => node.span(),
      CalleeNode::Root | CalleeNode::Module => Span::default(),
    }
  }
}

impl PartialEq for CalleeNode<'_> {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (CalleeNode::Module, CalleeNode::Module) => true,
      (CalleeNode::Function(a), CalleeNode::Function(b)) => a.span() == b.span(),
      (CalleeNode::ArrowFunctionExpression(a), CalleeNode::ArrowFunctionExpression(b)) => {
        a.span() == b.span()
      }
      (CalleeNode::ClassStatics(a), CalleeNode::ClassStatics(b)) => a.span() == b.span(),
      _ => false,
    }
  }
}

impl Eq for CalleeNode<'_> {}

impl hash::Hash for CalleeNode<'_> {
  fn hash<H: hash::Hasher>(&self, state: &mut H) {
    self.span().hash(state)
  }
}

#[derive(Debug, Clone, Copy)]
pub struct CalleeInfo<'a> {
  pub module_id: ModuleId,
  pub node: CalleeNode<'a>,
  pub instance_id: usize,
  #[cfg(feature = "flame")]
  pub debug_name: &'a str,
}

impl<'a> CalleeInfo<'a> {
  pub fn into_node(self) -> AstKind2<'a> {
    self.node.into()
  }

  pub fn span(&self) -> Span {
    self.node.span()
  }

  pub fn name(&self) -> &'a str {
    match self.node {
      CalleeNode::Function(node) => node.id.as_ref().map_or("<unknown>", |id| &id.name),
      CalleeNode::ArrowFunctionExpression(_) => "<anonymous>",
      CalleeNode::ClassStatics(_) => "<ClassStatics>",
      CalleeNode::ClassConstructor(_) => "<ClassConstructor>",
      CalleeNode::Root | CalleeNode::Module => "<Module>",
    }
  }
}

impl<'a> Analyzer<'a> {
  pub fn new_callee_info(&self, node: CalleeNode<'a>) -> CalleeInfo<'a> {
    CalleeInfo {
      module_id: self.current_module(),
      node,
      instance_id: self.factory.alloc_instance_id(),
      #[cfg(feature = "flame")]
      debug_name: {
        let line_col = self.line_index().line_col(node.span().start.into());
        let resolved_name = match node {
          CalleeNode::Function(node) => {
            if let Some(id) = &node.id {
              &id.name
            } else {
              self.resolve_function_name(node.scope_id()).unwrap_or("<unnamed>")
            }
          }
          CalleeNode::ArrowFunctionExpression(node) => {
            self.resolve_function_name(node.scope_id()).unwrap_or("<anonymous>")
          }
          CalleeNode::ClassStatics(_) => "<ClassStatics>",
          CalleeNode::ClassConstructor(_) => "<ClassConstructor>",
          CalleeNode::Root => "<Root>",
          CalleeNode::Module => "<Module>",
        };
        let debug_name = format!("{}:{}:{}", resolved_name, line_col.line + 1, line_col.col + 1);
        self.allocator.alloc(debug_name)
      },
    }
  }
}
