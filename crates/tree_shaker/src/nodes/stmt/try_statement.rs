use crate::{analyzer::Analyzer, transformer::Transformer};
use oxc::ast::ast::{Statement, TryStatement};

impl<'a> Analyzer<'a> {
  pub fn exec_try_statement(&mut self, node: &'a TryStatement<'a>) {
    self.push_try_scope();
    self.exec_block_statement(&node.block);
    let try_scope = self.pop_try_scope();

    let uncaught = if let Some(handler) = &node.handler {
      self.exec_catch_clause(
        handler,
        // Theoretically, if `thrown_val` is `None`, it means that the `try` block
        // does not throw any value, so we should skip the `catch` block.
        // However, we can guarantee that all possible exceptions tracked.
        // For example, KeyboardInterrupt, which is not tracked, can be thrown.
        try_scope.thrown_val(self).unwrap_or_else(|| self.factory.unknown()),
      );
      None
    } else {
      try_scope.may_throw.then_some(try_scope.thrown_values)
    };

    if let Some(finalizer) = &node.finalizer {
      self.exec_block_statement(finalizer);
    }

    if !self.cf_scope().must_exited() {
      if let Some(uncaught) = uncaught {
        self.forward_throw(uncaught);
      }
    }
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_try_statement(&self, node: &'a TryStatement<'a>) -> Option<Statement<'a>> {
    let TryStatement { span, block, handler, finalizer } = node;

    let block = self.transform_block_statement(block);

    let handler_span = handler.as_ref().map(|handler| handler.span).unwrap_or_default();
    let handler = if block.is_some() {
      handler.as_ref().map(|handler| self.transform_catch_clause(handler))
    } else {
      None
    };

    let finalizer =
      finalizer.as_ref().and_then(|finalizer| self.transform_block_statement(finalizer));

    match (block, finalizer) {
      (None, None) => None,
      (None, Some(finalizer)) => Some(Statement::BlockStatement(finalizer)),
      (Some(block), finalizer) => Some(self.ast_builder.statement_try(
        *span,
        block,
        if finalizer.is_some() {
          handler
        } else {
          Some(handler.unwrap_or_else(|| {
            self.ast_builder.catch_clause(
              handler_span,
              None,
              self.ast_builder.block_statement(handler_span, self.ast_builder.vec()),
            )
          }))
        },
        finalizer,
      )),
    }
  }
}
