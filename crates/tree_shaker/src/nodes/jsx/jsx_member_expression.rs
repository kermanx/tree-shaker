use crate::{analyzer::Analyzer, ast::AstKind2, entity::Entity, transformer::Transformer};
use oxc::{
  allocator,
  ast::ast::{Expression, JSXMemberExpression},
};

impl<'a> Analyzer<'a> {
  pub fn exec_jsx_member_expression(&mut self, node: &'a JSXMemberExpression<'a>) -> Entity<'a> {
    let object = self.exec_jsx_member_expression_object(&node.object);
    let key = self.factory.string(&node.property.name);
    object.get_property(self, AstKind2::JSXMemberExpression(node), key)
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_jsx_member_expression_effect_only(
    &self,
    node: &'a JSXMemberExpression<'a>,
    need_val: bool,
  ) -> Option<Expression<'a>> {
    let JSXMemberExpression { span, object, property } = node;

    let need_access = need_val || self.is_referred(AstKind2::JSXMemberExpression(node));
    if need_access {
      let object = self.transform_jsx_member_expression_object_effect_only(object, true).unwrap();
      Some(Expression::from(self.ast_builder.member_expression_static(
        *span,
        object,
        self.ast_builder.identifier_name(property.span, property.name),
        false,
      )))
    } else {
      self.transform_jsx_member_expression_object_effect_only(object, false)
    }
  }

  pub fn transform_jsx_member_expression_need_val(
    &self,
    node: &'a JSXMemberExpression<'a>,
  ) -> allocator::Box<'a, JSXMemberExpression<'a>> {
    let JSXMemberExpression { span, object, property } = node;

    self.ast_builder.alloc_jsx_member_expression(
      *span,
      self.transform_jsx_member_expression_object_need_val(object),
      self.clone_node(property),
    )
  }
}
