use oxc::{
  ast::ast::{Expression, Statement, SwitchCase, SwitchStatement},
  span::Span,
};

use crate::{
  analyzer::Analyzer, ast::AstKind2, scope::CfScopeKind, transformer::Transformer,
  utils::StatementVecData,
};

impl<'a> Analyzer<'a> {
  pub fn exec_switch_statement(&mut self, node: &'a SwitchStatement<'a>) {
    self.set_variable_scope_depth(node.scope_id());

    // 1. discriminant
    let discriminant = self.exec_expression(&node.discriminant);
    self.push_dependent_cf_scope(discriminant);

    // 2. tests
    let mut default_case = None;
    let mut maybe_default_case = Some(true);
    let mut test_results = vec![];
    let mut indeterminate = false;
    for (index, case) in node.cases.iter().enumerate() {
      if let Some(test) = &case.test {
        let test_val = self.exec_expression(test);

        // TODO: Support mangling
        let (test_result, m) = self.op_strict_eq(discriminant, test_val);
        test_results.push(test_result);

        if test_result != Some(false) {
          self.consume(AstKind2::SwitchCaseTest(case));
        }

        match test_result {
          Some(true) => {
            maybe_default_case = Some(false);
            break;
          }
          Some(false) => {}
          None => {
            self.consume((discriminant, test_val));
            if let Some(m) = m {
              m.add_to_mangler(&mut self.mangler);
            }
            // data.need_test.insert(index);
            maybe_default_case = None;
            if !indeterminate {
              indeterminate = true;
              self.push_indeterminate_cf_scope();
            }
          }
        }
      } else {
        default_case = Some(index);
        test_results.push(/* Updated later */ None);
      }
    }
    if indeterminate {
      self.pop_cf_scope();
    }

    // Patch default case
    if let Some(default_case) = default_case {
      test_results[default_case] = maybe_default_case;
      if maybe_default_case != Some(false) {
        self.consume(AstKind2::SwitchCaseTest(&node.cases[default_case]));
      }
    }

    // 3. consequent
    self.push_cf_scope(CfScopeKind::Switch, Some(false));
    let mut entered = Some(false);
    for (index, case) in node.cases.iter().enumerate() {
      if self.cf_scope().must_exited() {
        break;
      }

      let test_result = test_results.get(index).unwrap_or(&Some(false));

      entered = match (test_result, entered) {
        (Some(true), Some(true)) => unreachable!(),
        (Some(true), _) => Some(true),
        (Some(false), prev) => prev,
        (None, Some(true)) => Some(true),
        (None, _) => None,
      };

      if entered != Some(false) {
        self.consume(AstKind2::SwitchCase(case));

        let data = self.load_data::<StatementVecData>(AstKind2::SwitchCase(case));

        if entered.is_none() {
          self.push_indeterminate_cf_scope();
        }
        self.exec_statement_vec(data, &case.consequent);
        if entered.is_none() {
          self.pop_cf_scope();
        }
      }
    }

    self.pop_cf_scope();
    self.pop_cf_scope();
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_switch_statement(&self, node: &'a SwitchStatement<'a>) -> Option<Statement<'a>> {
    let SwitchStatement { span, discriminant, cases, .. } = node;

    let mut transformed_cases: Vec<(
      Span,
      Option<Expression<'a>>,
      oxc::allocator::Vec<'a, Statement<'a>>,
    )> = vec![];
    for case in cases {
      let need_test = self.is_referred(AstKind2::SwitchCaseTest(case));
      let need_consequent = self.is_referred(AstKind2::SwitchCase(case));

      let data = self.get_data::<StatementVecData>(AstKind2::SwitchCase(case));

      let SwitchCase { test, consequent, .. } = case;

      let test = test.as_ref().map(|test| {
        if need_test || self.transform_expression(test, false).is_some() {
          self.transform_expression(test, true)
        } else {
          None
        }
      });

      let consequent = if need_consequent {
        self.transform_statement_vec(data, consequent)
      } else {
        self.ast_builder.vec()
      };

      match test {
        Some(None) => {
          if consequent.len() > 0 {
            if let Some(last) = transformed_cases.last_mut() {
              last.2.extend(consequent);
            } else {
              // In case the first case is default + no consequent
              transformed_cases.push((*span, None, consequent));
            }
          }
        }
        Some(Some(test)) => {
          transformed_cases.push((*span, Some(test), consequent));
        }
        None => {
          if need_test {
            transformed_cases.push((*span, None, consequent));
          }
        }
      }
    }

    if transformed_cases.is_empty() {
      self
        .transform_expression(discriminant, false)
        .map(|expr| self.ast_builder.statement_expression(*span, expr))
    } else {
      let discriminant = self.transform_expression(discriminant, true).unwrap();

      Some(self.ast_builder.statement_switch(*span, discriminant, {
        let mut cases = self.ast_builder.vec();
        for (span, test, consequent) in transformed_cases {
          cases.push(self.ast_builder.switch_case(span, test, consequent));
        }
        cases
      }))
    }
  }
}
