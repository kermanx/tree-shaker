use crate::{
  builtins::{Builtins, constants::SYMBOL_CONSTRUCTOR_OBJECT_ID},
  entity::{ObjectPropertyValue, ObjectPrototype},
  init_namespace,
};
use std::borrow::BorrowMut;

impl Builtins<'_> {
  pub fn init_symbol_constructor(&mut self) {
    let factory = self.factory;

    let object = factory.builtin_object(
      SYMBOL_CONSTRUCTOR_OBJECT_ID,
      ObjectPrototype::Builtin(&self.prototypes.function),
      false,
    );
    object.init_rest(factory, ObjectPropertyValue::Field(factory.immutable_unknown, true));

    init_namespace!(object, factory, {
      "prototype" => factory.immutable_unknown,
      // "asyncIterator" => factory.string("__#asyncIterator__"),
      // "hasInstance" => factory.string("__#hasInstance__"),
      // "isConcatSpreadable" => factory.string("__#isConcatSpreadable__"),
      // "iterator" => factory.string("__#iterator__"),
      // "match" => factory.string("__#match__"),
      // "matchAll" => factory.string("__#matchAll__"),
      // "replace" => factory.string("__#replace__"),
      // "search" => factory.string("__#search__"),
      // "species" => factory.string("__#species__"),
      // "split" => factory.string("__#split__"),
      // "toPrimitive" => factory.string("__#toPrimitive__"),
      // "toStringTag" => factory.string("__#toStringTag__"),
      // "unscopables" => factory.string("__#unscopables__"),
      // "toString" => factory.string("__#toString__"),
    });

    self.globals.borrow_mut().insert("Symbol", object.into());
  }
}
