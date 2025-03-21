use oxc::allocator::{self, Allocator};

use super::{try_scope::TryScope, variable_scope::VariableScopeId};
use crate::{
  analyzer::Analyzer, consumable::ConsumeTrait, dep::DepId, entity::Entity, utils::CalleeInfo,
};
use std::mem;

pub struct CallScope<'a> {
  pub call_id: DepId,
  pub callee: CalleeInfo<'a>,
  pub old_variable_scope_stack: Vec<VariableScopeId>,
  pub cf_scope_depth: usize,
  pub body_variable_scope: VariableScopeId,
  pub returned_values: Vec<Entity<'a>>,
  pub is_async: bool,
  pub is_generator: bool,
  pub try_scopes: Vec<TryScope<'a>>,
  pub need_consume_arguments: bool,

  #[cfg(feature = "flame")]
  pub scope_guard: flame::SpanGuard,
}

impl<'a> CallScope<'a> {
  pub fn new_in(
    call_id: DepId,
    callee: CalleeInfo<'a>,
    old_variable_scope_stack: Vec<VariableScopeId>,
    cf_scope_depth: usize,
    body_variable_scope: VariableScopeId,
    is_async: bool,
    is_generator: bool,
    allocator: &'a Allocator,
  ) -> Self {
    CallScope {
      call_id,
      callee,
      old_variable_scope_stack,
      cf_scope_depth,
      body_variable_scope,
      returned_values: Vec::new(),
      is_async,
      is_generator,
      try_scopes: vec![TryScope::new_in(cf_scope_depth, allocator)],
      need_consume_arguments: false,

      #[cfg(feature = "flame")]
      scope_guard: flame::start_guard(callee.debug_name.to_string()),
    }
  }

  pub fn finalize(self, analyzer: &mut Analyzer<'a>) -> (Vec<VariableScopeId>, Entity<'a>) {
    assert_eq!(self.try_scopes.len(), 1);

    // Forwards the thrown value to the parent try scope
    let try_scope = self.try_scopes.into_iter().next().unwrap();
    let mut promise_error = None;
    if try_scope.may_throw {
      if self.is_generator {
        let unknown = analyzer.factory.unknown();
        let parent_try_scope = analyzer.try_scope_mut();
        parent_try_scope.may_throw = true;
        if !try_scope.thrown_values.is_empty() {
          parent_try_scope.thrown_values.push(unknown);
        }
        analyzer.consume(try_scope.thrown_values);
      } else if self.is_async {
        promise_error = Some(try_scope.thrown_values);
      } else {
        analyzer.forward_throw(try_scope.thrown_values);
      }
    }

    let value = if self.returned_values.is_empty() {
      analyzer.factory.undefined
    } else {
      analyzer.factory.union(allocator::Vec::from_iter_in(
        self.returned_values.iter().copied(),
        analyzer.allocator,
      ))
    };

    let value =
      if self.is_async { analyzer.factory.computed_unknown((value, promise_error)) } else { value };

    #[cfg(feature = "flame")]
    self.scope_guard.end();

    (self.old_variable_scope_stack, value)
  }
}

impl<'a> Analyzer<'a> {
  pub fn return_value(&mut self, value: Entity<'a>, dep: impl ConsumeTrait<'a> + 'a) {
    let call_scope = self.call_scope();
    let exec_dep = self.get_exec_dep(call_scope.cf_scope_depth);
    let value = self.factory.computed(value, (exec_dep, dep));

    let call_scope = self.call_scope_mut();
    call_scope.returned_values.push(value);

    let target_depth = call_scope.cf_scope_depth;
    self.exit_to(target_depth);
  }

  pub fn consume_arguments(&mut self) -> bool {
    let scope = self.call_scope().body_variable_scope;
    self.consume_arguments_on_scope(scope)
  }

  pub fn consume_return_values(&mut self) {
    let call_scope = self.call_scope_mut();
    let values = mem::take(&mut call_scope.returned_values);
    for value in values {
      self.consume(value);
    }
  }
}
