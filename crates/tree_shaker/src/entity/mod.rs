mod arguments;
mod array;
mod builtin_fn;
mod consumed_object;
mod factory;
mod function;
mod literal;
mod logical_result;
mod never;
mod object;
mod primitive;
mod react_element;
mod typeof_result;
mod union;
mod unknown;
mod utils;

use crate::{
  analyzer::Analyzer,
  consumable::{Consumable, ConsumableTrait, ConsumeTrait},
};
pub use builtin_fn::PureBuiltinFnEntity;
pub use factory::EntityFactory;
pub use literal::LiteralEntity;
pub use object::{
  ObjectEntity, ObjectId, ObjectProperty, ObjectPropertyId, ObjectPropertyValue, ObjectPrototype,
};
use oxc::allocator;
pub use primitive::PrimitiveEntity;
use rustc_hash::FxHashSet;
use std::{cmp::Ordering, fmt::Debug};
pub use typeof_result::TypeofResult;
pub use unknown::UnknownEntity;
pub use utils::*;

/// (vec![(definite, key, value)], dep)
pub type EnumeratedProperties<'a> = (Vec<(bool, Entity<'a>, Entity<'a>)>, Consumable<'a>);

/// (vec![known_elements], rest, dep)
pub type IteratedElements<'a> = (Vec<Entity<'a>>, Option<Entity<'a>>, Consumable<'a>);

pub trait ValueTrait<'a>: Debug {
  fn consume(&'a self, analyzer: &mut Analyzer<'a>);
  /// Returns true if the entity is completely consumed
  fn consume_mangable(&'a self, analyzer: &mut Analyzer<'a>) -> bool {
    self.consume(analyzer);
    true
  }
  fn unknown_mutate(&'a self, analyzer: &mut Analyzer<'a>, dep: Consumable<'a>);

  fn get_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    key: Entity<'a>,
  ) -> Entity<'a>;
  fn set_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    key: Entity<'a>,
    value: Entity<'a>,
  );
  fn enumerate_properties(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
  ) -> EnumeratedProperties<'a>;
  fn delete_property(&'a self, analyzer: &mut Analyzer<'a>, dep: Consumable<'a>, key: Entity<'a>);
  fn call(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    this: Entity<'a>,
    args: Entity<'a>,
  ) -> Entity<'a>;
  fn construct(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    args: Entity<'a>,
  ) -> Entity<'a>;
  fn jsx(&'a self, analyzer: &mut Analyzer<'a>, props: Entity<'a>) -> Entity<'a>;
  fn r#await(&'a self, analyzer: &mut Analyzer<'a>, dep: Consumable<'a>) -> Entity<'a>;
  fn iterate(&'a self, analyzer: &mut Analyzer<'a>, dep: Consumable<'a>) -> IteratedElements<'a>;

  fn get_typeof(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a>;
  fn get_to_string(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a>;
  fn get_to_numeric(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a>;
  fn get_to_boolean(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a>;
  fn get_to_property_key(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a>;
  fn get_to_jsx_child(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a>;
  fn get_to_literals(&'a self, _analyzer: &Analyzer<'a>) -> Option<FxHashSet<LiteralEntity<'a>>> {
    None
  }
  fn get_literal(&'a self, analyzer: &Analyzer<'a>) -> Option<LiteralEntity<'a>> {
    self
      .get_to_literals(analyzer)
      .and_then(|set| if set.len() == 1 { set.into_iter().next() } else { None })
  }
  /// Returns vec![(definite, key)]
  fn get_own_keys(&'a self, _analyzer: &Analyzer<'a>) -> Option<Vec<(bool, Entity<'a>)>> {
    None
  }
  fn get_constructor_prototype(
    &'a self,
    _analyzer: &Analyzer<'a>,
    _dep: Consumable<'a>,
  ) -> Option<(Consumable<'a>, ObjectPrototype<'a>, ObjectPrototype<'a>)> {
    None
  }

  fn test_typeof(&self) -> TypeofResult;
  fn test_truthy(&self) -> Option<bool>;
  fn test_nullish(&self) -> Option<bool>;
  fn test_is_undefined(&self) -> Option<bool> {
    let t = self.test_typeof();
    match (t == TypeofResult::Undefined, t.contains(TypeofResult::Undefined)) {
      (true, _) => Some(true),
      (false, true) => None,
      (false, false) => Some(false),
    }
  }

  fn destruct_as_array(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    length: usize,
    need_rest: bool,
  ) -> (Vec<Entity<'a>>, Option<Entity<'a>>, Consumable<'a>) {
    let (mut elements, rest, dep) = self.iterate(analyzer, dep);
    let iterated_len = elements.len();
    let extras = match iterated_len.cmp(&length) {
      Ordering::Equal => Vec::new(),
      Ordering::Greater => elements.split_off(length),
      Ordering::Less => {
        elements.resize(length, rest.unwrap_or(analyzer.factory.undefined));
        Vec::new()
      }
    };
    for element in &mut elements {
      *element = analyzer.factory.computed(*element, dep);
    }

    let rest_arr = need_rest.then(|| {
      let rest_arr = analyzer.new_empty_array();
      rest_arr.deps.borrow_mut().push(if extras.is_empty() && rest.is_none() {
        analyzer.consumable((self, dep))
      } else {
        dep
      });
      rest_arr.elements.borrow_mut().extend(extras);
      if let Some(rest) = rest {
        rest_arr.init_rest(rest);
      }
      rest_arr.into()
    });

    (elements, rest_arr, dep)
  }

  fn iterate_result_union(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
  ) -> Option<Entity<'a>> {
    let (elements, rest, deps) = self.iterate(analyzer, dep);
    if let Some(rest) = rest {
      let mut result = allocator::Vec::from_iter_in(elements.iter().copied(), analyzer.allocator);
      result.push(rest);
      Some(analyzer.factory.computed_union(result, deps))
    } else if !elements.is_empty() {
      Some(analyzer.factory.computed_union(
        allocator::Vec::from_iter_in(elements.iter().copied(), analyzer.allocator),
        deps,
      ))
    } else {
      None
    }
  }

  fn call_as_getter(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    this: Entity<'a>,
  ) -> Entity<'a> {
    self.call(analyzer, dep, this, analyzer.factory.empty_arguments)
  }

  fn call_as_setter(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    this: Entity<'a>,
    value: Entity<'a>,
  ) -> Entity<'a> {
    self.call(
      analyzer,
      dep,
      this,
      analyzer.factory.arguments(analyzer.factory.vec1((false, value))),
    )
  }
}

impl<'a, T: ValueTrait<'a> + 'a + ?Sized> ConsumableTrait<'a> for &'a T {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    (*self).consume(analyzer)
  }
}

pub type Value<'a> = &'a (dyn ValueTrait<'a> + 'a);

#[derive(Debug, Clone, Copy)]
pub struct Entity<'a> {
  value: Value<'a>,
  dep: Option<Consumable<'a>>,
}

impl<'a> Entity<'a> {
  pub fn shallow_dep(&self) -> Option<Consumable<'a>> {
    self.dep
  }

  fn forward_dep(
    &self,
    dep: impl ConsumeTrait<'a> + 'a,
    analyzer: &Analyzer<'a>,
  ) -> Consumable<'a> {
    if let Some(d) = self.dep {
      analyzer.factory.consumable((d, dep))
    } else {
      dep.uniform(analyzer.factory.allocator)
    }
  }

  fn forward_value(&self, entity: Entity<'a>, analyzer: &Analyzer<'a>) -> Entity<'a> {
    Entity {
      value: entity.value,
      dep: match (self.dep, entity.dep) {
        (Some(d1), Some(d2)) => Some(analyzer.factory.consumable((d1, d2))),
        (Some(d), None) | (None, Some(d)) => Some(d),
        (None, None) => None,
      },
    }
  }

  pub fn consume(&self, analyzer: &mut Analyzer<'a>) {
    analyzer.consume(*self);
  }

  /// Returns true if the entity is completely consumed
  pub fn consume_mangable(&self, analyzer: &mut Analyzer<'a>) -> bool {
    analyzer.consume(self.dep);
    self.value.consume_mangable(analyzer)
  }

  pub fn unknown_mutate(&self, analyzer: &mut Analyzer<'a>, dep: impl ConsumeTrait<'a> + 'a) {
    self.value.unknown_mutate(analyzer, self.forward_dep(dep, analyzer));
  }

  pub fn get_property(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: impl ConsumeTrait<'a> + 'a,
    key: Entity<'a>,
  ) -> Entity<'a> {
    self.value.get_property(analyzer, self.forward_dep(dep, analyzer), key)
  }
  pub fn set_property(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: impl ConsumeTrait<'a> + 'a,
    key: Entity<'a>,
    value: Entity<'a>,
  ) {
    self.value.set_property(analyzer, self.forward_dep(dep, analyzer), key, value)
  }
  pub fn enumerate_properties(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: impl ConsumeTrait<'a> + 'a,
  ) -> EnumeratedProperties<'a> {
    self.value.enumerate_properties(analyzer, self.forward_dep(dep, analyzer))
  }
  pub fn delete_property(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: impl ConsumeTrait<'a> + 'a,
    key: Entity<'a>,
  ) {
    self.value.delete_property(analyzer, self.forward_dep(dep, analyzer), key)
  }
  pub fn call(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: impl ConsumeTrait<'a> + 'a,
    this: Entity<'a>,
    args: Entity<'a>,
  ) -> Entity<'a> {
    self.value.call(analyzer, self.forward_dep(dep, analyzer), this, args)
  }
  pub fn construct(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: impl ConsumeTrait<'a> + 'a,
    args: Entity<'a>,
  ) -> Entity<'a> {
    self.value.construct(analyzer, self.forward_dep(dep, analyzer), args)
  }
  pub fn jsx(&self, analyzer: &mut Analyzer<'a>, props: Entity<'a>) -> Entity<'a> {
    self.forward_value(self.value.jsx(analyzer, props), analyzer)
  }
  pub fn r#await(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: impl ConsumeTrait<'a> + 'a,
  ) -> Entity<'a> {
    self.forward_value(self.value.r#await(analyzer, self.forward_dep(dep, analyzer)), analyzer)
  }
  pub fn iterate(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: impl ConsumeTrait<'a> + 'a,
  ) -> IteratedElements<'a> {
    self.value.iterate(analyzer, self.forward_dep(dep, analyzer))
  }
  pub fn get_typeof(&self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.forward_value(self.value.get_typeof(analyzer), analyzer)
  }
  pub fn get_to_string(&self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.forward_value(self.value.get_to_string(analyzer), analyzer)
  }
  pub fn get_to_numeric(&self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.forward_value(self.value.get_to_numeric(analyzer), analyzer)
  }
  pub fn get_to_boolean(&self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.forward_value(self.value.get_to_boolean(analyzer), analyzer)
  }
  pub fn get_to_property_key(&self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.forward_value(self.value.get_to_property_key(analyzer), analyzer)
  }
  pub fn get_to_jsx_child(&self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.forward_value(self.value.get_to_jsx_child(analyzer), analyzer)
  }
  pub fn get_to_literals(&self, analyzer: &Analyzer<'a>) -> Option<FxHashSet<LiteralEntity<'a>>> {
    self.value.get_to_literals(analyzer)
  }
  pub fn get_literal(&self, analyzer: &Analyzer<'a>) -> Option<LiteralEntity<'a>> {
    self.value.get_literal(analyzer)
  }
  /// Returns vec![(definite, key)]
  pub fn get_own_keys(&self, analyzer: &Analyzer<'a>) -> Option<Vec<(bool, Entity<'a>)>> {
    self.value.get_own_keys(analyzer)
  }
  pub fn get_constructor_prototype(
    &self,
    analyzer: &Analyzer<'a>,
    dep: impl ConsumeTrait<'a> + 'a,
  ) -> Option<(Consumable<'a>, ObjectPrototype<'a>, ObjectPrototype<'a>)> {
    self.value.get_constructor_prototype(analyzer, self.forward_dep(dep, analyzer))
  }
  pub fn test_typeof(&self) -> TypeofResult {
    self.value.test_typeof()
  }
  pub fn test_truthy(&self) -> Option<bool> {
    self.value.test_truthy()
  }
  pub fn test_nullish(&self) -> Option<bool> {
    self.value.test_nullish()
  }
  pub fn test_is_undefined(&self) -> Option<bool> {
    self.value.test_is_undefined()
  }

  pub fn destruct_as_array(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: impl ConsumeTrait<'a> + 'a,
    length: usize,
    need_rest: bool,
  ) -> (Vec<Entity<'a>>, Option<Entity<'a>>, Consumable<'a>) {
    self.value.destruct_as_array(analyzer, self.forward_dep(dep, analyzer), length, need_rest)
  }

  pub fn iterate_result_union(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: impl ConsumeTrait<'a> + 'a,
  ) -> Option<Entity<'a>> {
    self.value.iterate_result_union(analyzer, self.forward_dep(dep, analyzer))
  }

  pub fn call_as_getter(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: impl ConsumeTrait<'a> + 'a,
    this: Entity<'a>,
  ) -> Entity<'a> {
    self.value.call_as_getter(analyzer, self.forward_dep(dep, analyzer), this)
  }

  pub fn call_as_setter(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: impl ConsumeTrait<'a> + 'a,
    this: Entity<'a>,
    value: Entity<'a>,
  ) -> Entity<'a> {
    self.value.call_as_setter(analyzer, self.forward_dep(dep, analyzer), this, value)
  }
}

impl<'a> ConsumableTrait<'a> for Entity<'a> {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    analyzer.consume(self.value);
    analyzer.consume(self.dep);
  }
}

impl<'a, T: ValueTrait<'a> + 'a> From<&'a T> for Entity<'a> {
  fn from(value: &'a T) -> Self {
    Entity { value, dep: None }
  }
}
impl<'a, T: ValueTrait<'a> + 'a> From<&'a mut T> for Entity<'a> {
  fn from(value: &'a mut T) -> Self {
    (&*value).into()
  }
}

impl<'a> EntityFactory<'a> {
  pub fn entity_with_dep(&self, value: Value<'a>, dep: Consumable<'a>) -> Entity<'a> {
    Entity { value, dep: Some(dep) }
  }

  pub fn computed(&self, entity: Entity<'a>, dep: impl ConsumeTrait<'a> + 'a) -> Entity<'a> {
    Entity {
      value: entity.value,
      dep: if let Some(d) = entity.dep {
        Some(self.consumable((d, dep)))
      } else {
        Some(dep.uniform(self.allocator))
      },
    }
  }
}
