use oxc::{allocator, ast::ast::BlockStatement};

use crate::{analyzer::Analyzer, ast::AstKind2, transformer::Transformer, utils::StatementVecData};

impl<'a> Analyzer<'a> {
  pub fn exec_block_statement(&mut self, node: &'a BlockStatement) {
    let data = self.load_data::<StatementVecData>(AstKind2::BlockStatement(node));

    self.set_variable_scope_depth(node.scope_id());

    self.exec_statement_vec(data, &node.body);
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_block_statement(
    &self,
    node: &'a BlockStatement<'a>,
  ) -> Option<allocator::Box<'a, BlockStatement<'a>>> {
    let data = self.get_data::<StatementVecData>(AstKind2::BlockStatement(node));

    let BlockStatement { span, body, .. } = node;

    let statements = self.transform_statement_vec(data, body);

    if statements.is_empty() {
      None
    } else {
      Some(self.ast_builder.alloc_block_statement(*span, statements))
    }
  }
}
