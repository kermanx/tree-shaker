use super::{
  consumed_object, never::NeverEntity, Entity, EntityTrait, EnumeratedProperties, IteratedElements,
  TypeofResult,
};
use crate::{analyzer::Analyzer, builtins::BuiltinPrototype, consumable::Consumable};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveEntity {
  // TODO: NumericString, NoneEmptyString, ...
  Mixed,
  String,
  Number,
  BigInt,
  Boolean,
  Symbol,
}

impl<'a> EntityTrait<'a> for PrimitiveEntity {
  fn consume(&'a self, _analyzer: &mut Analyzer<'a>) {}

  fn unknown_mutate(&'a self, _analyzer: &mut Analyzer<'a>, _dep: Consumable<'a>) {
    // No effect
  }

  fn get_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    key: Entity<'a>,
  ) -> Entity<'a> {
    // TODO: PrimitiveEntity::String
    if *self == PrimitiveEntity::Mixed || *self == PrimitiveEntity::String {
      analyzer.factory.computed_unknown((self, dep, key))
    } else {
      let prototype = self.get_prototype(analyzer);
      prototype.get_property(analyzer, self, key, dep)
    }
  }

  fn set_property(
    &'a self,
    _analyzer: &mut Analyzer<'a>,
    _dep: Consumable<'a>,
    _key: Entity<'a>,
    _value: Entity<'a>,
  ) {
    // No effect
  }

  fn enumerate_properties(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
  ) -> EnumeratedProperties<'a> {
    if *self == PrimitiveEntity::String {
      (
        vec![(false, analyzer.factory.unknown_string, analyzer.factory.unknown_string)],
        analyzer.consumable((self, dep)),
      )
    } else {
      (vec![], analyzer.consumable((self, dep)))
    }
  }

  fn delete_property(
    &'a self,
    _analyzer: &mut Analyzer<'a>,
    _dep: Consumable<'a>,
    _key: Entity<'a>,
  ) {
    // No effect
  }

  fn call(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    this: Entity<'a>,
    args: Entity<'a>,
  ) -> Entity<'a> {
    analyzer.throw_builtin_error("Cannot call non-object");
    if analyzer.config.preserve_exceptions {
      consumed_object::call(self, analyzer, dep, this, args)
    } else {
      analyzer.factory.never
    }
  }

  fn construct(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    args: Entity<'a>,
  ) -> Entity<'a> {
    analyzer.throw_builtin_error("Cannot construct non-object");
    if analyzer.config.preserve_exceptions {
      consumed_object::construct(self, analyzer, dep, args)
    } else {
      analyzer.factory.never
    }
  }

  fn jsx(&'a self, analyzer: &mut Analyzer<'a>, props: Entity<'a>) -> Entity<'a> {
    analyzer.factory.computed_unknown((self, props))
  }

  fn r#await(&'a self, analyzer: &mut Analyzer<'a>, dep: Consumable<'a>) -> Entity<'a> {
    analyzer.factory.computed(self, dep)
  }

  fn iterate(&'a self, analyzer: &mut Analyzer<'a>, dep: Consumable<'a>) -> IteratedElements<'a> {
    if *self == PrimitiveEntity::String {
      return (vec![], Some(analyzer.factory.unknown()), analyzer.consumable((self, dep)));
    }
    analyzer.throw_builtin_error("Cannot iterate non-object");
    if analyzer.config.preserve_exceptions {
      self.consume(analyzer);
      consumed_object::iterate(analyzer, dep)
    } else {
      NeverEntity.iterate(analyzer, dep)
    }
  }

  fn get_destructable(&'a self, analyzer: &Analyzer<'a>, dep: Consumable<'a>) -> Consumable<'a> {
    analyzer.consumable((self, dep))
  }

  fn get_typeof(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    if let Some(str) = self.test_typeof().to_string() {
      analyzer.factory.string(str)
    } else {
      analyzer.factory.unknown_string
    }
  }

  fn get_to_string(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.unknown_string
  }

  fn get_to_numeric(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.unknown()
  }

  fn get_to_boolean(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    match self.test_truthy() {
      Some(val) => analyzer.factory.boolean(val),
      None => analyzer.factory.unknown_boolean,
    }
  }

  fn get_to_property_key(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.unknown()
  }

  fn get_to_jsx_child(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    if matches!(self, PrimitiveEntity::Mixed | PrimitiveEntity::String | PrimitiveEntity::Number) {
      analyzer.factory.unknown_string
    } else {
      analyzer.factory.string("")
    }
  }
  fn get_own_keys(&'a self, _analyzer: &Analyzer<'a>) -> Option<Vec<(bool, Entity<'a>)>> {
    match self {
      PrimitiveEntity::String => None,
      _ => Some(vec![]),
    }
  }

  fn test_typeof(&self) -> TypeofResult {
    match self {
      PrimitiveEntity::String => TypeofResult::String,
      PrimitiveEntity::Number => TypeofResult::Number,
      PrimitiveEntity::BigInt => TypeofResult::BigInt,
      PrimitiveEntity::Boolean => TypeofResult::Boolean,
      PrimitiveEntity::Symbol => TypeofResult::Symbol,
      PrimitiveEntity::Mixed => TypeofResult::_Unknown,
    }
  }

  fn test_truthy(&self) -> Option<bool> {
    match self {
      PrimitiveEntity::Symbol => Some(true),
      _ => None,
    }
  }

  fn test_nullish(&self) -> Option<bool> {
    Some(false)
  }
}

impl<'a> PrimitiveEntity {
  fn get_prototype(&self, analyzer: &mut Analyzer<'a>) -> &'a BuiltinPrototype<'a> {
    match self {
      PrimitiveEntity::String => &analyzer.builtins.prototypes.string,
      PrimitiveEntity::Number => &analyzer.builtins.prototypes.number,
      PrimitiveEntity::BigInt => &analyzer.builtins.prototypes.bigint,
      PrimitiveEntity::Boolean => &analyzer.builtins.prototypes.boolean,
      PrimitiveEntity::Symbol => &analyzer.builtins.prototypes.symbol,
      PrimitiveEntity::Mixed => unreachable!("Cannot get prototype of mixed primitive"),
    }
  }
}
