use oxc::ast::ast::{Expression, IdentifierName};
use oxc_ast_visit::VisitMut;

use crate::{
  transformer::Transformer,
  utils::{ast::AstKind2, dep_id::DepId},
};

use super::MangleAtom;

pub struct ManglerTransformer<'a>(pub Transformer<'a>);

impl<'a> VisitMut<'a> for ManglerTransformer<'a> {
  fn visit_identifier_name(&mut self, node: &mut IdentifierName<'a>) {
    if let Some(atom) = self.0.get_data::<Option<MangleAtom>>(AstKind2::IdentifierName(node)) {
      let mut mangler = self.0.mangler.borrow_mut();
      if let Some(mangled) = mangler.resolve(*atom) {
        node.name = mangled.into();
      }
    }
  }

  fn visit_expression(&mut self, node: &mut Expression<'a>) {
    if let Some(folded) = self.0.build_folded_expr(DepId::from(AstKind2::Expression(node)).into()) {
      *node = folded;
    }
  }
}
