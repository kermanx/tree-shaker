pub mod call_scope;
pub mod cf_scope;
pub mod conditional;
pub mod exhaustive;
// pub mod r#loop;
mod scope_tree;
pub mod try_scope;
mod utils;
pub mod variable_scope;

use crate::{
  analyzer::Analyzer,
  consumable::{Consumable, ConsumableVec, ConsumeTrait},
  dep::DepId,
  entity::{Entity, EntityFactory, ObjectId},
  module::ModuleId,
  utils::{CalleeInfo, CalleeNode},
};
use call_scope::CallScope;
use cf_scope::CfScope;
pub use cf_scope::{CfScopeId, CfScopeKind};
use scope_tree::ScopeTree;
use try_scope::TryScope;
use variable_scope::VariableScope;
pub use variable_scope::VariableScopeId;

pub struct Scoping<'a> {
  pub call: Vec<CallScope<'a>>,
  pub variable: ScopeTree<VariableScopeId, VariableScope<'a>>,
  pub cf: ScopeTree<CfScopeId, CfScope<'a>>,
  pub pure: usize,

  pub object_scope_id: VariableScopeId,
  pub object_symbol_counter: usize,
}

impl<'a> Scoping<'a> {
  pub fn new(factory: &EntityFactory<'a>) -> Self {
    let mut variable = ScopeTree::new();
    variable.push(VariableScope::new_with_this(factory.unknown()));
    let object_scope_id = variable.add_special(VariableScope::new());
    let mut cf = ScopeTree::new();
    cf.push(CfScope::new(CfScopeKind::Root, factory.vec(), Some(false)));
    Scoping {
      call: vec![CallScope::new_in(
        DepId::from_counter(),
        CalleeInfo {
          module_id: ModuleId::from(0),
          node: CalleeNode::Root,
          instance_id: factory.alloc_instance_id(),
          #[cfg(feature = "flame")]
          debug_name: "<Module>",
        },
        vec![],
        0,
        VariableScopeId::from(0),
        false,
        false,
        factory.allocator,
      )],
      variable,
      cf,
      pure: 0,

      object_scope_id,
      object_symbol_counter: 128,
    }
  }

  pub fn assert_final_state(&mut self) {
    assert_eq!(self.call.len(), 1);
    assert_eq!(self.variable.current_depth(), 0);
    assert_eq!(self.cf.current_depth(), 1);
    assert_eq!(self.pure, 0);

    for scope in self.cf.iter_all() {
      if let CfScopeKind::Exhaustive(data) = &scope.kind {
        assert!(data.clean);
      }
    }

    #[cfg(feature = "flame")]
    self.call.pop().unwrap().scope_guard.end();
  }

  pub fn alloc_object_id(&mut self) -> ObjectId {
    self.object_symbol_counter += 1;
    ObjectId::from_usize(self.object_symbol_counter)
  }
}

impl<'a> Analyzer<'a> {
  pub fn call_scope(&self) -> &CallScope<'a> {
    self.scoping.call.last().unwrap()
  }

  pub fn call_scope_mut(&mut self) -> &mut CallScope<'a> {
    self.scoping.call.last_mut().unwrap()
  }

  pub fn try_scope(&self) -> &TryScope<'a> {
    self.call_scope().try_scopes.last().unwrap()
  }

  pub fn try_scope_mut(&mut self) -> &mut TryScope<'a> {
    self.call_scope_mut().try_scopes.last_mut().unwrap()
  }

  pub fn cf_scope(&self) -> &CfScope<'a> {
    self.scoping.cf.get_current()
  }

  pub fn cf_scope_mut(&mut self) -> &mut CfScope<'a> {
    self.scoping.cf.get_current_mut()
  }

  pub fn cf_scope_id_of_call_scope(&self) -> CfScopeId {
    let depth = self.call_scope().cf_scope_depth;
    self.scoping.cf.stack[depth]
  }

  pub fn variable_scope(&self) -> &VariableScope<'a> {
    self.scoping.variable.get_current()
  }

  pub fn variable_scope_mut(&mut self) -> &mut VariableScope<'a> {
    self.scoping.variable.get_current_mut()
  }

  pub fn is_inside_pure(&self) -> bool {
    // TODO: self.scoping.pure > 0
    false
  }

  pub fn replace_variable_scope_stack(
    &mut self,
    new_stack: Vec<VariableScopeId>,
  ) -> Vec<VariableScopeId> {
    self.scoping.variable.replace_stack(new_stack)
  }

  pub fn push_call_scope(
    &mut self,
    callee: CalleeInfo<'a>,
    call_dep: Consumable<'a>,
    variable_scope_stack: Vec<VariableScopeId>,
    is_async: bool,
    is_generator: bool,
    consume: bool,
  ) {
    let dep_id = DepId::from_counter();
    if consume {
      self.refer_dep(dep_id);
    }

    self.module_stack.push(callee.module_id);
    let old_variable_scope_stack = self.replace_variable_scope_stack(variable_scope_stack);
    let body_variable_scope = self.push_variable_scope();
    let cf_scope_depth = self.push_cf_scope_with_deps(
      CfScopeKind::Function,
      self.factory.vec1(self.consumable((call_dep, dep_id))),
      Some(false),
    );

    self.scoping.call.push(CallScope::new_in(
      dep_id,
      callee,
      old_variable_scope_stack,
      cf_scope_depth,
      body_variable_scope,
      is_async,
      is_generator,
      self.allocator,
    ));
  }

  pub fn pop_call_scope(&mut self) -> Entity<'a> {
    let scope = self.scoping.call.pop().unwrap();
    let (old_variable_scope_stack, ret_val) = scope.finalize(self);
    self.pop_cf_scope();
    self.pop_variable_scope();
    self.replace_variable_scope_stack(old_variable_scope_stack);
    self.module_stack.pop();
    ret_val
  }

  pub fn push_variable_scope(&mut self) -> VariableScopeId {
    self.scoping.variable.push(VariableScope::new())
  }

  pub fn pop_variable_scope(&mut self) -> VariableScopeId {
    self.scoping.variable.pop()
  }

  pub fn push_cf_scope(&mut self, kind: CfScopeKind<'a>, exited: Option<bool>) -> usize {
    self.push_cf_scope_with_deps(kind, self.factory.vec(), exited)
  }

  pub fn push_cf_scope_with_deps(
    &mut self,
    kind: CfScopeKind<'a>,
    deps: ConsumableVec<'a>,
    exited: Option<bool>,
  ) -> usize {
    self.scoping.cf.push(CfScope::new(kind, deps, exited));
    self.scoping.cf.current_depth()
  }

  pub fn push_indeterminate_cf_scope(&mut self) {
    self.push_cf_scope(CfScopeKind::Indeterminate, None);
  }

  pub fn push_dependent_cf_scope(&mut self, dep: impl ConsumeTrait<'a> + 'a) {
    self.push_cf_scope_with_deps(
      CfScopeKind::Dependent,
      self.factory.vec1(dep.uniform(self.allocator)),
      Some(false),
    );
  }

  pub fn pop_cf_scope(&mut self) -> CfScopeId {
    self.scoping.cf.pop()
  }

  pub fn pop_multiple_cf_scopes(&mut self, count: usize) -> Option<Consumable<'a>> {
    let mut exec_deps = self.factory.vec();
    for _ in 0..count {
      let id = self.scoping.cf.stack.pop().unwrap();
      if let Some(dep) = self.scoping.cf.get_mut(id).deps.try_collect(self.factory) {
        exec_deps.push(dep);
      }
    }
    if exec_deps.is_empty() { None } else { Some(self.consumable(exec_deps)) }
  }

  pub fn pop_cf_scope_and_get_mut(&mut self) -> &mut CfScope<'a> {
    let id = self.pop_cf_scope();
    self.scoping.cf.get_mut(id)
  }

  pub fn push_try_scope(&mut self) {
    self.push_indeterminate_cf_scope();
    let cf_scope_depth = self.scoping.cf.current_depth();
    let try_scope = TryScope::new_in(cf_scope_depth, self.allocator);
    self.call_scope_mut().try_scopes.push(try_scope);
  }

  pub fn pop_try_scope(&mut self) -> TryScope<'a> {
    self.pop_cf_scope();
    self.call_scope_mut().try_scopes.pop().unwrap()
  }
}
