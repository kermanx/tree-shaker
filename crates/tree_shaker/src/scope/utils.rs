use crate::{
  analyzer::Analyzer,
  consumable::{ConsumableCollector, ConsumableVec},
  entity::ObjectId,
};
use std::mem;

use super::{cf_scope::CfScopeId, exhaustive::ExhaustiveDepId};

impl<'a> Analyzer<'a> {
  pub fn find_first_different_cf_scope(&self, another: CfScopeId) -> usize {
    self.scoping.cf.find_lca(another).0 + 1
  }

  /// Returns (has_exhaustive, indeterminate, exec_deps)
  pub fn pre_mutate_object(
    &mut self,
    cf_scope: CfScopeId,
    object_id: ObjectId,
  ) -> (bool, bool, ConsumableVec<'a>) {
    let target_depth = self.find_first_different_cf_scope(cf_scope);

    let mut has_exhaustive = false;
    let mut indeterminate = false;
    let mut exec_deps = self.factory.vec();
    for depth in target_depth..self.scoping.cf.stack.len() {
      let scope = self.scoping.cf.get_mut_from_depth(depth);
      if !has_exhaustive {
        has_exhaustive |= scope.mark_exhaustive_write(ExhaustiveDepId::Object(object_id));
      }
      indeterminate |= scope.is_indeterminate();
      if let Some(dep) = scope.deps.try_collect(self.factory) {
        exec_deps.push(dep);
      }
    }

    self.request_exhaustive_callbacks(ExhaustiveDepId::Object(object_id));

    (has_exhaustive, indeterminate, exec_deps)
  }

  pub fn mark_object_property_exhaustive_read(&mut self, cf_scope: CfScopeId, object_id: ObjectId) {
    let target_depth = self.find_first_different_cf_scope(cf_scope);
    self.mark_exhaustive_read(ExhaustiveDepId::Object(object_id), target_depth);
  }

  pub fn mark_object_consumed(&mut self, cf_scope: CfScopeId, object_id: ObjectId) {
    let factory = self.factory;
    let target_depth = self.find_first_different_cf_scope(cf_scope);
    let mut marked = false;
    for depth in target_depth..self.scoping.cf.stack.len() {
      let scope = self.scoping.cf.get_mut_from_depth(depth);
      if !marked {
        marked = scope.mark_exhaustive_write(ExhaustiveDepId::Object(object_id));
      }
      let deps = mem::replace(&mut scope.deps, ConsumableCollector::new(factory.vec()));
      deps.consume_all(self);
    }
    self.request_exhaustive_callbacks(ExhaustiveDepId::Object(object_id));
  }
}
