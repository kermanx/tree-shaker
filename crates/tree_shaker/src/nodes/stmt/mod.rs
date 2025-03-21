mod block_statement;
mod break_statement;
mod continue_statement;
mod declaration;
mod do_while_statement;
mod for_in_statement;
mod for_of_statement;
mod for_statement;
mod if_statement;
mod labeled_statements;
mod module_declaration;
mod return_statement;
mod statement_vec;
mod switch_statement;
mod throw_statement;
mod try_statement;
mod while_statement;

use crate::{Analyzer, transformer::Transformer};
use oxc::{
  ast::{
    ast::{ExpressionStatement, Statement},
    match_declaration, match_module_declaration,
  },
  span::GetSpan,
};

impl<'a> Analyzer<'a> {
  pub fn declare_statement(&mut self, node: &'a Statement) {
    self.push_span(node);
    match node {
      match_declaration!(Statement) => {
        let node = node.to_declaration();
        self.declare_declaration(node, false);
      }
      match_module_declaration!(Statement) => {
        let node = node.to_module_declaration();
        self.declare_module_declaration(node);
      }
      Statement::LabeledStatement(node) => self.declare_labeled_statement(node),
      _ => {}
    }
    self.pop_span();
  }

  pub fn init_statement(&mut self, node: &'a Statement) {
    self.push_span(node);
    match node {
      match_declaration!(Statement) => {
        let node = node.to_declaration();
        self.init_declaration(node);
      }
      match_module_declaration!(Statement) => {
        let node = node.to_module_declaration();
        self.init_module_declaration(node);
      }
      Statement::ExpressionStatement(node) => {
        self.exec_expression(&node.expression);
      }
      Statement::BlockStatement(node) => self.exec_block_statement(node),
      Statement::IfStatement(node) => self.exec_if_statement(node),
      Statement::WhileStatement(node) => self.exec_while_statement(node),
      Statement::DoWhileStatement(node) => self.exec_do_while_statement(node),
      Statement::ForStatement(node) => self.exec_for_statement(node),
      Statement::ForInStatement(node) => self.exec_for_in_statement(node),
      Statement::ForOfStatement(node) => self.exec_for_of_statement(node),
      Statement::SwitchStatement(node) => self.exec_switch_statement(node),
      Statement::BreakStatement(node) => self.exec_break_statement(node),
      Statement::ContinueStatement(node) => self.exec_continue_statement(node),
      Statement::ReturnStatement(node) => self.exec_return_statement(node),
      Statement::LabeledStatement(node) => self.exec_labeled_statement(node),
      Statement::TryStatement(node) => self.exec_try_statement(node),
      Statement::ThrowStatement(node) => self.exec_throw_statement(node),
      Statement::EmptyStatement(_) => {}
      Statement::DebuggerStatement(_node) => {}
      Statement::WithStatement(_node) => unimplemented!("with statement"),
    }
    self.pop_span();
  }

  pub fn exec_statement(&mut self, node: &'a Statement) {
    self.declare_statement(node);
    self.init_statement(node);
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_statement(&self, node: &'a Statement<'a>) -> Option<Statement<'a>> {
    let span = node.span();
    match node {
      match_declaration!(Statement) => {
        self.transform_declaration(node.to_declaration()).map(Statement::from)
      }
      match_module_declaration!(Statement) => {
        self.transform_module_declaration(node.to_module_declaration()).map(Statement::from)
      }
      Statement::ExpressionStatement(node) => {
        let ExpressionStatement { expression, .. } = node.as_ref();
        self
          .transform_expression(expression, false)
          .map(|expr| self.ast_builder.statement_expression(span, expr))
      }
      Statement::BlockStatement(node) => {
        self.transform_block_statement(node).map(Statement::BlockStatement)
      }
      Statement::IfStatement(node) => self.transform_if_statement(node),
      Statement::WhileStatement(node) => self.transform_while_statement(node),
      Statement::DoWhileStatement(node) => self.transform_do_while_statement(node),
      Statement::ForStatement(node) => self.transform_for_statement(node),
      Statement::ForInStatement(node) => self.transform_for_in_statement(node),
      Statement::ForOfStatement(node) => self.transform_for_of_statement(node),
      Statement::SwitchStatement(node) => self.transform_switch_statement(node),
      Statement::BreakStatement(node) => self.transform_break_statement(node),
      Statement::ContinueStatement(node) => self.transform_continue_statement(node),
      Statement::ReturnStatement(node) => self.transform_return_statement(node),
      Statement::LabeledStatement(node) => self.transform_labeled_statement(node),
      Statement::TryStatement(node) => self.transform_try_statement(node),
      Statement::ThrowStatement(node) => self.transform_throw_statement(node),
      Statement::EmptyStatement(_) => None,
      Statement::DebuggerStatement(node) => Some(self.ast_builder.statement_debugger(node.span())),
      Statement::WithStatement(_node) => unreachable!(),
    }
  }
}
