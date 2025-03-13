mod array;
mod bigint;
mod boolean;
mod function;
mod null;
mod number;
mod object;
mod promise;
mod regexp;
mod string;
mod symbol;
mod utils;

use std::fmt;

use crate::{
  analyzer::Analyzer,
  consumable::Consumable,
  entity::{Entity, EntityFactory, LiteralEntity},
};
use oxc::semantic::SymbolId;
use rustc_hash::FxHashMap;

use super::Builtins;

#[derive(Default)]
pub struct BuiltinPrototype<'a> {
  name: &'static str,
  string_keyed: FxHashMap<&'static str, Entity<'a>>,
  symbol_keyed: FxHashMap<SymbolId, Entity<'a>>,
}

impl<'a> fmt::Debug for BuiltinPrototype<'a> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(format!("Prototype({})", self.name).as_str())
  }
}

impl<'a> BuiltinPrototype<'a> {
  pub fn with_name(mut self, name: &'static str) -> Self {
    self.name = name;
    self
  }

  pub fn insert_string_keyed(&mut self, key: &'static str, value: impl Into<Entity<'a>>) {
    self.string_keyed.insert(key, value.into());
  }

  pub fn insert_symbol_keyed(&mut self, key: SymbolId, value: impl Into<Entity<'a>>) {
    self.symbol_keyed.insert(key, value.into());
  }

  pub fn get_string_keyed(&self, key: &str) -> Option<Entity<'a>> {
    self.string_keyed.get(key).copied()
  }

  pub fn get_symbol_keyed(&self, key: SymbolId) -> Option<Entity<'a>> {
    self.symbol_keyed.get(&key).copied()
  }

  pub fn get_literal_keyed(&self, key: LiteralEntity) -> Option<Entity<'a>> {
    match key {
      LiteralEntity::String(key, _) => self.get_string_keyed(key),
      LiteralEntity::Symbol(key, _) => self.get_symbol_keyed(key),
      _ => unreachable!("Invalid property key"),
    }
  }

  pub fn get_property(
    &self,
    analyzer: &Analyzer<'a>,
    target: Entity<'a>,
    key: Entity<'a>,
    dep: Consumable<'a>,
  ) -> Entity<'a> {
    let dep = analyzer.consumable((dep, target, key));
    if let Some(key_literals) = key.get_to_literals(analyzer) {
      let mut values = vec![];
      for key_literal in key_literals {
        if let Some(property) = self.get_literal_keyed(key_literal) {
          values.push(property);
        } else {
          values.push(analyzer.factory.unmatched_prototype_property);
        }
      }
      analyzer.factory.computed_union(values, dep)
    } else {
      analyzer.factory.computed_unknown(dep)
    }
  }
}

pub struct BuiltinPrototypes<'a> {
  pub array: BuiltinPrototype<'a>,
  pub bigint: BuiltinPrototype<'a>,
  pub boolean: BuiltinPrototype<'a>,
  pub function: BuiltinPrototype<'a>,
  pub null: BuiltinPrototype<'a>,
  pub number: BuiltinPrototype<'a>,
  pub object: BuiltinPrototype<'a>,
  pub promise: BuiltinPrototype<'a>,
  pub regexp: BuiltinPrototype<'a>,
  pub string: BuiltinPrototype<'a>,
  pub symbol: BuiltinPrototype<'a>,
}

impl<'a> Builtins<'a> {
  pub fn create_builtin_prototypes(factory: &EntityFactory<'a>) -> &'a BuiltinPrototypes<'a> {
    factory.alloc(BuiltinPrototypes {
      array: array::create_array_prototype(factory),
      bigint: bigint::create_bigint_prototype(factory),
      boolean: boolean::create_boolean_prototype(factory),
      function: function::create_function_prototype(factory),
      null: null::create_null_prototype(factory),
      number: number::create_number_prototype(factory),
      object: object::create_object_prototype(factory),
      promise: promise::create_promise_prototype(factory),
      regexp: regexp::create_regexp_prototype(factory),
      string: string::create_string_prototype(factory),
      symbol: symbol::create_symbol_prototype(factory),
    })
  }
}
