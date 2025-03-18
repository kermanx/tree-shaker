use super::{cf_scope::ReferredState, VariableScopeId};
use crate::{
  analyzer::Analyzer,
  entity::{Entity, ObjectId, ObjectPropertyId},
  scope::CfScopeKind,
};
use oxc::semantic::SymbolId;
use rustc_hash::FxHashSet;
use std::{
  hash::{Hash, Hasher},
  mem,
  rc::Rc,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExhaustiveDepId {
  Variable(VariableScopeId, SymbolId),
  Object(ObjectId),
  ObjectProperty(ObjectId, ObjectPropertyId),
}

#[derive(Debug)]
pub struct ExhaustiveData {
  pub clean: bool,
  pub temp_deps: Option<FxHashSet<ExhaustiveDepId>>,
  pub register_deps: Option<FxHashSet<ExhaustiveDepId>>,
}

#[derive(Clone)]
pub struct ExhaustiveCallback<'a> {
  pub handler: Rc<dyn Fn(&mut Analyzer<'a>) + 'a>,
  pub once: bool,
}
impl<'a> PartialEq for ExhaustiveCallback<'a> {
  fn eq(&self, other: &Self) -> bool {
    self.once == other.once && Rc::ptr_eq(&self.handler, &other.handler)
  }
}
impl<'a> Eq for ExhaustiveCallback<'a> {}
impl Hash for ExhaustiveCallback<'_> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    Rc::as_ptr(&self.handler).hash(state);
  }
}

impl<'a> Analyzer<'a> {
  pub fn exec_loop(&mut self, runner: impl Fn(&mut Analyzer<'a>) + 'a) {
    let runner = Rc::new(runner);

    self.exec_exhaustively("loop", false, false, runner.clone());

    let cf_scope = self.cf_scope();
    if cf_scope.referred_state != ReferredState::ReferredClean && cf_scope.deps.may_not_referred() {
      self.push_indeterminate_cf_scope();
      runner(self);
      self.pop_cf_scope();
    }
  }

  pub fn exec_consumed_fn(
    &mut self,
    kind: &str,
    runner: impl Fn(&mut Analyzer<'a>) -> Entity<'a> + 'a,
  ) {
    let runner: Rc<dyn Fn(&mut Analyzer<'a>) + 'a> = Rc::new(move |analyzer| {
      analyzer.push_indeterminate_cf_scope();
      analyzer.push_try_scope();
      let ret_val = runner(analyzer);
      let thrown_val = analyzer.pop_try_scope().thrown_val(analyzer);
      if !analyzer.is_inside_pure() {
        analyzer.consume(ret_val);
        analyzer.consume(thrown_val);
      }
      analyzer.pop_cf_scope();
    });
    self.exec_exhaustively(kind, false, true, runner);
  }

  pub fn exec_async_or_generator_fn(&mut self, runner: impl Fn(&mut Analyzer<'a>) + 'a) {
    self.exec_exhaustively("async/generator", true, true, Rc::new(runner));
  }

  fn exec_exhaustively(
    &mut self,
    _kind: &str,
    once: bool,
    register: bool,
    runner: Rc<dyn Fn(&mut Analyzer<'a>) + 'a>,
  ) {
    self.push_cf_scope(
      CfScopeKind::Exhaustive(ExhaustiveData {
        clean: true,
        temp_deps: (!once).then(FxHashSet::default),
        register_deps: register.then(Default::default),
      }),
      Some(false),
    );
    let mut round_counter = 0;
    loop {
      #[cfg(feature = "flame")]
      let _scope_guard = flame::start_guard(format!(
        "!{_kind}@{:06X} x{}",
        (Rc::as_ptr(&runner) as *const () as usize) & 0xFFFFFF,
        round_counter
      ));
      runner(self);
      round_counter += 1;
      if round_counter > 1000 {
        unreachable!("Exhaustive loop is too deep");
      }
      if !self.cf_scope_mut().post_exhaustive_iterate() {
        break;
      }
    }
    let id = self.pop_cf_scope();
    let data = self.scoping.cf.get_mut(id).exhaustive_data_mut().unwrap();
    if register {
      let deps = data.register_deps.take().unwrap();
      self.register_exhaustive_callbacks(once, runner, deps);
    }
  }

  fn register_exhaustive_callbacks(
    &mut self,
    once: bool,
    handler: Rc<dyn Fn(&mut Analyzer<'a>) + 'a>,
    deps: FxHashSet<ExhaustiveDepId>,
  ) {
    for id in deps {
      self
        .exhaustive_callbacks
        .entry(id)
        .or_default()
        .insert(ExhaustiveCallback { handler: handler.clone(), once });
    }
  }

  pub fn mark_exhaustive_read(&mut self, id: ExhaustiveDepId, target: usize) {
    let mut registered = false;
    for depth in (target..self.scoping.cf.stack.len()).rev() {
      let scope = self.scoping.cf.get_mut_from_depth(depth);
      if let Some(data) = scope.exhaustive_data_mut() {
        if data.clean {
          if let Some(temp_deps) = data.temp_deps.as_mut() {
            temp_deps.insert(id);
          }
        }
        if !registered {
          if let Some(register_deps) = data.register_deps.as_mut() {
            registered = true;
            register_deps.insert(id);
          }
        }
      }
    }
  }

  pub fn mark_exhaustive_write(&mut self, id: ExhaustiveDepId, target: usize) -> (bool, bool) {
    let mut should_consume = false;
    let mut indeterminate = false;
    for depth in target..self.scoping.cf.stack.len() {
      let scope = self.scoping.cf.get_mut_from_depth(depth);
      if !should_consume {
        should_consume |= scope.mark_exhaustive_write(id);
      }
      indeterminate |= scope.is_indeterminate();
    }
    (should_consume, indeterminate)
  }

  pub fn request_exhaustive_callbacks(
    &mut self,
    _should_consume: bool,
    id: ExhaustiveDepId,
  ) -> bool {
    if let Some(runners) = self.exhaustive_callbacks.get_mut(&id) {
      if runners.is_empty() {
        false
      } else {
        self.pending_deps.extend(runners.drain());
        true
      }
    } else {
      false
    }
  }

  pub fn call_exhaustive_callbacks(&mut self) -> bool {
    if self.pending_deps.is_empty() {
      return false;
    }
    loop {
      let runners = mem::take(&mut self.pending_deps);
      for runner in runners {
        // let old_count = self.referred_deps.debug_count();
        let ExhaustiveCallback { handler: runner, once } = runner;
        self.exec_exhaustively("dep", once, true, runner.clone());
        // let new_count = self.referred_deps.debug_count();
        // self.debug += 1;
      }
      if self.pending_deps.is_empty() {
        return true;
      }
    }
  }

  pub fn has_exhaustive_scope_since(&self, target_depth: usize) -> bool {
    self.scoping.cf.iter_stack_range(target_depth..).any(|scope| scope.kind.is_exhaustive())
  }
}
