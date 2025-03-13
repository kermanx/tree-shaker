use super::{
  consumed_object, utils::UnionLike, Entity, EntityTrait, EnumeratedProperties, IteratedElements,
  LiteralEntity, ObjectPrototype, TypeofResult,
};
use crate::{analyzer::Analyzer, consumable::Consumable, scope::CfScopeKind, use_consumed_flag};
use rustc_hash::FxHashSet;
use std::{cell::Cell, fmt::Debug};

#[derive(Debug)]
pub struct UnionEntity<'a, V: UnionLike<'a, Entity<'a>> + Debug + 'a> {
  /// Possible values
  pub values: V,
  pub consumed: Cell<bool>,
  pub phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, V: UnionLike<'a, Entity<'a>> + Debug + 'a> EntityTrait<'a> for UnionEntity<'a, V> {
  fn consume(&'a self, analyzer: &mut Analyzer<'a>) {
    use_consumed_flag!(self);

    for value in self.values.iter() {
      value.consume(analyzer);
    }
  }

  fn consume_mangable(&'a self, analyzer: &mut Analyzer<'a>) -> bool {
    if !self.consumed.get() {
      let mut consumed = true;
      for value in self.values.iter() {
        consumed &= value.consume_mangable(analyzer);
      }
      self.consumed.set(consumed);
      consumed
    } else {
      true
    }
  }

  fn unknown_mutate(&'a self, analyzer: &mut Analyzer<'a>, dep: Consumable<'a>) {
    for value in self.values.iter() {
      value.unknown_mutate(analyzer, dep);
    }
  }

  fn get_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    key: Entity<'a>,
  ) -> Entity<'a> {
    let values = analyzer
      .exec_indeterminately(|analyzer| self.values.map(|v| v.get_property(analyzer, dep, key)));
    analyzer.factory.union(values)
  }

  fn set_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    key: Entity<'a>,
    value: Entity<'a>,
  ) {
    analyzer.exec_indeterminately(|analyzer| {
      for entity in self.values.iter() {
        entity.set_property(analyzer, dep, key, value)
      }
    });
  }

  fn enumerate_properties(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
  ) -> EnumeratedProperties<'a> {
    // FIXME:
    consumed_object::enumerate_properties(self, analyzer, dep)
  }

  fn delete_property(&'a self, analyzer: &mut Analyzer<'a>, dep: Consumable<'a>, key: Entity<'a>) {
    analyzer.exec_indeterminately(|analyzer| {
      for entity in self.values.iter() {
        entity.delete_property(analyzer, dep, key);
      }
    })
  }

  fn call(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    this: Entity<'a>,
    args: Entity<'a>,
  ) -> Entity<'a> {
    analyzer.push_cf_scope_with_deps(CfScopeKind::Dependent, vec![analyzer.consumable(self)], None);
    let values = self.values.map(|v| v.call(analyzer, dep, this, args));
    analyzer.pop_cf_scope();
    analyzer.factory.union(values)
  }

  fn construct(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    args: Entity<'a>,
  ) -> Entity<'a> {
    let values = analyzer
      .exec_indeterminately(|analyzer| self.values.map(|v| v.construct(analyzer, dep, args)));
    analyzer.factory.union(values)
  }

  fn jsx(&'a self, analyzer: &mut Analyzer<'a>, props: Entity<'a>) -> Entity<'a> {
    let values =
      analyzer.exec_indeterminately(|analyzer| self.values.map(|v| v.jsx(analyzer, props)));
    analyzer.factory.union(values)
  }

  fn r#await(&'a self, analyzer: &mut Analyzer<'a>, dep: Consumable<'a>) -> Entity<'a> {
    let values =
      analyzer.exec_indeterminately(|analyzer| self.values.map(|v| v.r#await(analyzer, dep)));
    analyzer.factory.union(values)
  }

  fn iterate(&'a self, analyzer: &mut Analyzer<'a>, dep: Consumable<'a>) -> IteratedElements<'a> {
    let mut results = Vec::new();
    let mut has_undefined = false;
    analyzer.push_indeterminate_cf_scope();
    for entity in self.values.iter() {
      if let Some(result) = entity.iterate_result_union(analyzer, dep) {
        results.push(result);
      } else {
        has_undefined = true;
      }
    }
    analyzer.pop_cf_scope();
    if has_undefined {
      results.push(analyzer.factory.undefined);
    }
    (vec![], analyzer.factory.try_union(results), analyzer.factory.empty_consumable)
  }

  fn get_destructable(&'a self, analyzer: &Analyzer<'a>, dep: Consumable<'a>) -> Consumable<'a> {
    let mut values = Vec::new();
    for entity in self.values.iter() {
      values.push(entity.get_destructable(analyzer, dep));
    }
    analyzer.consumable(values)
  }

  fn get_typeof(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    // TODO: collect literals
    let values = self.values.map(|v| v.get_typeof(analyzer));
    analyzer.factory.union(values)
  }

  fn get_to_string(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    // TODO: dedupe
    let values = self.values.map(|v| v.get_to_string(analyzer));
    analyzer.factory.union(values)
  }

  fn get_to_numeric(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    // TODO: dedupe
    let values = self.values.map(|v| v.get_to_numeric(analyzer));
    analyzer.factory.union(values)
  }

  fn get_to_boolean(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    let values = self.values.map(|v| v.get_to_boolean(analyzer));
    analyzer.factory.union(values)
  }

  fn get_to_property_key(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    let values = self.values.map(|v| v.get_to_property_key(analyzer));
    analyzer.factory.union(values)
  }

  fn get_to_jsx_child(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    let values = self.values.map(|v| v.get_to_jsx_child(analyzer));
    analyzer.factory.union(values)
  }

  fn get_to_literals(&'a self, analyzer: &Analyzer<'a>) -> Option<FxHashSet<LiteralEntity<'a>>> {
    let mut iter = self.values.iter();
    let mut result = iter.next().unwrap().get_to_literals(analyzer)?;
    for entity in iter {
      result.extend(entity.get_to_literals(analyzer)?);
    }
    Some(result)
  }

  fn get_own_keys(&'a self, _analyzer: &Analyzer<'a>) -> Option<Vec<(bool, Entity<'a>)>> {
    let mut result = Vec::new();
    for entity in self.values.iter() {
      let keys = entity.get_own_keys(_analyzer)?;
      result.extend(keys.into_iter().map(|(_, key)| (false, key)));
    }
    Some(result)
  }

  fn get_constructor_prototype(
    &'a self,
    _analyzer: &Analyzer<'a>,
    _dep: Consumable<'a>,
  ) -> Option<(Consumable<'a>, ObjectPrototype<'a>, ObjectPrototype<'a>)> {
    // TODO: Support this
    None
  }

  fn test_typeof(&self) -> TypeofResult {
    let mut result = TypeofResult::_None;
    for entity in self.values.iter() {
      result |= entity.test_typeof();
    }
    result
  }

  fn test_truthy(&self) -> Option<bool> {
    let mut iter = self.values.iter();
    let result = iter.next().unwrap().test_truthy()?;
    for entity in iter {
      if entity.test_truthy()? != result {
        return None;
      }
    }
    Some(result)
  }

  fn test_nullish(&self) -> Option<bool> {
    let mut iter = self.values.iter();
    let result = iter.next().unwrap().test_nullish()?;
    for entity in iter {
      if entity.test_nullish()? != result {
        return None;
      }
    }
    Some(result)
  }
}
