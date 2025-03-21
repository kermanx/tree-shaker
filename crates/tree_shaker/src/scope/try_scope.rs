use oxc::allocator::{self, Allocator};

use crate::{analyzer::Analyzer, entity::Entity};

#[derive(Debug)]
pub struct TryScope<'a> {
  pub may_throw: bool,
  pub thrown_values: allocator::Vec<'a, Entity<'a>>,
  /// Here we use index in current stack instead of ScopeId
  pub cf_scope_depth: usize,
}

impl<'a> TryScope<'a> {
  pub fn new_in(cf_scope_depth: usize, allocator: &'a Allocator) -> Self {
    TryScope { may_throw: false, thrown_values: allocator::Vec::new_in(allocator), cf_scope_depth }
  }

  pub fn thrown_val(self, analyzer: &Analyzer<'a>) -> Option<Entity<'a>> {
    // Always unknown here
    self.may_throw.then(|| {
      if self.thrown_values.is_empty() {
        analyzer.factory.unknown()
      } else {
        analyzer.factory.computed_unknown(self.thrown_values)
      }
    })
  }
}

impl<'a> Analyzer<'a> {
  pub fn may_throw(&mut self) {
    let try_scope = self.try_scope_mut();

    try_scope.may_throw = true;

    // FIXME: Some of the tests are failing because of this
    // let cf_scope_depth = try_scope.cf_scope_depth;
    // self.exit_to(cf_scope_depth, false);
  }

  pub fn explicit_throw(&mut self, value: Entity<'a>) {
    self.explicit_throw_impl(value);

    let try_scope = self.try_scope();
    self.exit_to(try_scope.cf_scope_depth);
  }

  pub fn throw_builtin_error(&mut self, message: impl Into<String>) {
    if self.scoping.cf.iter_stack().all(|scope| scope.exited == Some(false)) {
      self.add_diagnostic(message);
    }

    self.explicit_throw_impl(self.factory.unknown());

    let try_scope = self.try_scope();
    self.exit_to(try_scope.cf_scope_depth);
  }

  pub fn forward_throw(&mut self, values: allocator::Vec<'a, Entity<'a>>) {
    if values.is_empty() {
      self.may_throw();
    } else {
      let thrown_val = self.factory.computed_unknown(values);
      self.explicit_throw_impl(thrown_val);

      let try_scope = self.try_scope();
      self.exit_to_not_must(try_scope.cf_scope_depth);
    }
  }

  fn explicit_throw_impl(&mut self, value: Entity<'a>) {
    let try_scope = self.try_scope();
    let exec_dep = self.get_exec_dep(try_scope.cf_scope_depth);
    let forwarded = self.factory.computed(value, exec_dep);

    let try_scope = self.try_scope_mut();
    try_scope.may_throw = true;
    try_scope.thrown_values.push(forwarded);
  }
}
