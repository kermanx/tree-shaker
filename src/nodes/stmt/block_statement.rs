use crate::{analyzer::Analyzer, ast::AstType2, transformer::Transformer};
use oxc::{
  ast::ast::{BlockStatement, Statement},
  span::{GetSpan, Span},
};

const AST_TYPE: AstType2 = AstType2::BlockStatement;

#[derive(Debug, Default)]
struct Data {
  last_stmt: Option<Span>,
}

impl<'a> Analyzer<'a> {
  pub(crate) fn exec_block_statement(&mut self, node: &'a BlockStatement) {
    self.push_variable_scope();
    self.push_cf_scope(Some(false));

    let mut span: Option<Span> = None;
    for statement in &node.body {
      if self.cf_scope().must_exited() {
        break;
      }
      self.exec_statement(statement);
      span = Some(statement.span());
    }
    if let Some(span) = span {
      let data = self.load_data::<Data>(AST_TYPE, node);
      data.last_stmt = match data.last_stmt {
        Some(current_span) => Some(current_span.max(span)),
        None => Some(span),
      };
    }

    self.pop_cf_scope();
    self.pop_variable_scope();
  }
}

impl<'a> Transformer<'a> {
  pub(crate) fn transform_block_statement(
    &mut self,
    node: BlockStatement<'a>,
  ) -> Option<Statement<'a>> {
    let data = self.get_data::<Data>(AST_TYPE, &node);

    let BlockStatement { span, body, .. } = node;

    let mut statements = self.ast_builder.vec();

    for statement in body {
      let span = statement.span();

      if let Some(statement) = self.transform_statement(statement) {
        statements.push(statement);
      }

      if data.last_stmt == Some(span) {
        break;
      }
    }

    if statements.is_empty() {
      None
    } else {
      Some(self.ast_builder.statement_block(span, statements))
    }
  }
}
