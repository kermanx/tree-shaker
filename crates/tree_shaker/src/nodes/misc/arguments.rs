use crate::{
  Analyzer,
  ast::{Arguments, AstKind2},
  entity::Entity,
  transformer::Transformer,
};
use oxc::{
  ast::ast::{Argument, Expression},
  span::GetSpan,
};

impl<'a> Analyzer<'a> {
  pub fn exec_arguments(&mut self, node: &'a Arguments<'a>) -> Entity<'a> {
    let mut arguments = self.factory.vec();
    for argument in node {
      let (spread, val) = match argument {
        Argument::SpreadElement(node) => (true, self.exec_expression(&node.argument)),
        node => (false, self.exec_expression(node.to_expression())),
      };
      let dep = AstKind2::Argument(argument);
      arguments.push((spread, self.factory.computed(val, dep)));
    }
    self.factory.arguments(arguments)
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_arguments_need_call(&self, node: &'a Arguments<'a>) -> Arguments<'a> {
    let mut arguments = self.ast_builder.vec();
    let mut preserve_args_num = false;
    for argument in node.into_iter().rev() {
      if let Some(argument) = self.transform_argument_need_call(argument, preserve_args_num) {
        arguments.insert(0, argument);
        preserve_args_num = true;
      }
    }
    arguments
  }

  fn transform_argument_need_call(
    &self,
    node: &'a Argument<'a>,
    preserve_args_num: bool,
  ) -> Option<Argument<'a>> {
    let is_referred = self.is_referred(AstKind2::Argument(node));
    let span = node.span();
    match node {
      Argument::SpreadElement(node) => {
        // Currently, a spread element de-optimize the arguments.
        let expr = self.transform_expression(&node.argument, true).unwrap();
        Some(self.ast_builder.argument_spread_element(span, expr))
      }
      _ => self
        .transform_expression(node.to_expression(), is_referred)
        .or_else(|| preserve_args_num.then(|| self.build_unused_expression(span)))
        .map(Argument::from),
    }
  }

  pub fn transform_arguments_no_call(
    &self,
    node: &'a Arguments<'a>,
  ) -> Vec<Option<Expression<'a>>> {
    node.into_iter().map(|arg| self.transform_argument_no_call(arg)).collect()
  }

  fn transform_argument_no_call(&self, node: &'a Argument<'a>) -> Option<Expression<'a>> {
    match node {
      Argument::SpreadElement(node) => self.transform_expression(&node.argument, false),
      _ => self.transform_expression(node.to_expression(), false),
    }
  }
}
