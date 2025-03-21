use super::CfScopeKind;
use crate::{
  analyzer::Analyzer,
  consumable::{Consumable, ConsumableTrait},
  dep::DepId,
  entity::Entity,
  transformer::Transformer,
};
use rustc_hash::FxHashMap;
use std::{cell::Cell, fmt::Debug, mem};

#[derive(Debug, Default)]
struct ConditionalData<'a> {
  maybe_true: bool,
  maybe_false: bool,
  impure_true: bool,
  impure_false: bool,
  tests_to_consume: Vec<Entity<'a>>,
}

#[derive(Debug, Default)]
pub struct ConditionalDataMap<'a> {
  call_to_branches: FxHashMap<DepId, Vec<&'a ConditionalBranch<'a>>>,
  node_to_data: FxHashMap<DepId, ConditionalData<'a>>,
}

#[derive(Debug, Clone)]
struct ConditionalBranch<'a> {
  dep_id: DepId,
  is_true_branch: bool,
  maybe_true: bool,
  maybe_false: bool,
  test: Entity<'a>,
  referred: &'a Cell<bool>,
}

impl<'a> ConditionalBranch<'a> {
  fn refer_with_data(&self, data: &mut ConditionalData<'a>) {
    if !self.referred.get() {
      self.referred.set(true);

      data.maybe_true |= self.maybe_true;
      data.maybe_false |= self.maybe_false;
      data.tests_to_consume.push(self.test);
      if self.is_true_branch {
        data.impure_true = true;
      } else {
        data.impure_false = true;
      }
    }
  }
}

impl<'a> ConsumableTrait<'a> for &'a ConditionalBranch<'a> {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    let data = analyzer.get_conditional_data_mut(self.dep_id);
    self.refer_with_data(data);
  }
}

impl<'a> Analyzer<'a> {
  #[allow(clippy::too_many_arguments)]
  pub fn push_if_like_branch_cf_scope(
    &mut self,
    dep_id: impl Into<DepId>,
    kind: CfScopeKind<'a>,
    test: Entity<'a>,
    maybe_consequent: bool,
    maybe_alternate: bool,
    is_consequent: bool,
    has_contra: bool,
  ) -> Consumable<'a> {
    let dep = self.push_conditional_cf_scope(
      dep_id,
      kind,
      test,
      maybe_consequent,
      maybe_alternate,
      is_consequent,
      has_contra,
    );
    self.consumable(dep)
  }

  pub fn forward_logical_left_val(
    &mut self,
    dep_id: impl Into<DepId>,
    left: Entity<'a>,
    maybe_left: bool,
    maybe_right: bool,
  ) -> Entity<'a> {
    assert!(maybe_left);
    let dep = self.register_conditional_data(dep_id, left, maybe_left, maybe_right, true, true);
    self.factory.computed(left, dep)
  }

  pub fn push_logical_right_cf_scope(
    &mut self,
    dep_id: impl Into<DepId>,
    left: Entity<'a>,
    maybe_left: bool,
    maybe_right: bool,
  ) -> Consumable<'a> {
    assert!(maybe_right);
    let dep = self.push_conditional_cf_scope(
      dep_id,
      CfScopeKind::Indeterminate,
      left,
      maybe_left,
      maybe_right,
      false,
      false,
    );
    self.consumable(dep)
  }

  #[allow(clippy::too_many_arguments)]
  fn push_conditional_cf_scope(
    &mut self,
    dep_id: impl Into<DepId>,
    kind: CfScopeKind<'a>,
    test: Entity<'a>,
    maybe_true: bool,
    maybe_false: bool,
    is_true: bool,
    has_contra: bool,
  ) -> impl ConsumableTrait<'a> + 'a {
    let dep =
      self.register_conditional_data(dep_id, test, maybe_true, maybe_false, is_true, has_contra);

    self.push_cf_scope_with_deps(
      kind,
      self.factory.vec1(self.consumable(dep)),
      if maybe_true && maybe_false { None } else { Some(false) },
    );

    dep
  }

  fn register_conditional_data(
    &mut self,
    dep_id: impl Into<DepId>,
    test: Entity<'a>,
    maybe_true: bool,
    maybe_false: bool,
    is_true: bool,
    has_contra: bool,
  ) -> &'a ConditionalBranch<'a> {
    let dep_id = dep_id.into();
    let call_id = self.call_scope().call_id;

    let branch = self.allocator.alloc(ConditionalBranch {
      dep_id,
      is_true_branch: is_true,
      maybe_true,
      maybe_false,
      test,
      referred: self.allocator.alloc(Cell::new(false)),
    });

    let ConditionalDataMap { call_to_branches, node_to_data } = &mut self.conditional_data;

    if has_contra {
      call_to_branches.entry(call_id).or_insert_with(Default::default).push(branch);
    }

    node_to_data.entry(dep_id).or_insert_with(ConditionalData::default);

    branch
  }

  fn is_contra_branch_impure(
    &mut self,
    branch: &'a ConditionalBranch<'a>,
  ) -> Option<&mut ConditionalData<'a>> {
    let data = self.get_conditional_data_mut(branch.dep_id);
    if branch.is_true_branch { data.impure_false } else { data.impure_true }.then_some(data)
  }

  pub fn post_analyze_handle_conditional(&mut self) -> bool {
    for (call_id, branches) in mem::take(&mut self.conditional_data.call_to_branches) {
      if self.is_referred(call_id) {
        let mut remaining_branches = vec![];
        for branch in branches {
          if let Some(data) = self.is_contra_branch_impure(branch) {
            branch.refer_with_data(data);
          } else {
            remaining_branches.push(branch);
          }
        }
        if !remaining_branches.is_empty() {
          self.conditional_data.call_to_branches.insert(call_id, remaining_branches);
        }
      } else {
        self.conditional_data.call_to_branches.insert(call_id, branches);
      }
    }

    let mut tests_to_consume = vec![];
    for data in self.conditional_data.node_to_data.values_mut() {
      if data.maybe_true && data.maybe_false {
        tests_to_consume.push(mem::take(&mut data.tests_to_consume));
      }
    }

    let mut dirty = false;
    for tests in tests_to_consume {
      for test in tests {
        test.consume(self);
        dirty = true;
      }
    }
    dirty
  }

  fn get_conditional_data_mut(&mut self, dep_id: DepId) -> &mut ConditionalData<'a> {
    self.conditional_data.node_to_data.get_mut(&dep_id).unwrap()
  }
}

impl Transformer<'_> {
  pub fn get_conditional_result(&self, dep_id: impl Into<DepId>) -> (bool, bool, bool) {
    let data = &self.conditional_data.node_to_data[&dep_id.into()];
    if data.maybe_true && data.maybe_false {
      assert!(data.tests_to_consume.is_empty());
    }
    (data.maybe_true && data.maybe_false, data.maybe_true, data.maybe_false)
  }

  pub fn get_chain_result(&self, dep_id: impl Into<DepId>, optional: bool) -> (bool, bool) {
    if optional {
      let (need_optional, _, may_not_short_circuit) = self.get_conditional_result(dep_id);
      (need_optional, !may_not_short_circuit)
    } else {
      (false, false)
    }
  }
}
