mod arguments;
mod array;
mod builtin_fn;
mod computed;
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
  consumable::{Consumable, ConsumableTrait},
};
pub use builtin_fn::PureBuiltinFnEntity;
pub use factory::EntityFactory;
pub use literal::LiteralEntity;
pub use object::{ObjectEntity, ObjectProperty, ObjectPropertyValue, ObjectPrototype};
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

pub trait EntityTrait<'a>: Debug {
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

  fn get_destructable(&'a self, analyzer: &Analyzer<'a>, dep: Consumable<'a>) -> Consumable<'a>;
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
    self.get_to_literals(analyzer).and_then(|set| {
      if set.len() == 1 {
        set.into_iter().next()
      } else {
        None
      }
    })
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
        analyzer.consumable(dep)
      });
      rest_arr.elements.borrow_mut().extend(extras);
      if let Some(rest) = rest {
        rest_arr.init_rest(rest);
      }
      rest_arr as Entity<'a>
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
      let mut result = elements;
      result.push(rest);
      Some(analyzer.factory.computed_union(result, deps))
    } else if !elements.is_empty() {
      Some(analyzer.factory.computed_union(elements, deps))
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
    self.call(analyzer, dep, this, analyzer.factory.arguments(vec![(false, value)]))
  }
}

pub type Entity<'a> = &'a (dyn EntityTrait<'a> + 'a);

impl<'a, T: EntityTrait<'a> + 'a + ?Sized> ConsumableTrait<'a> for &'a T {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    (*self).consume(analyzer)
  }
}
