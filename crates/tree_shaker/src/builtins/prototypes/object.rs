use super::{BuiltinPrototype, null::create_null_prototype};
use crate::{entity::EntityFactory, init_prototype};

pub fn create_object_prototype<'a>(factory: &EntityFactory<'a>) -> BuiltinPrototype<'a> {
  init_prototype!("Object", create_null_prototype(factory), {
    "constructor" => factory.immutable_unknown,
    "hasOwnProperty" => factory.pure_fn_returns_boolean,
    "isPrototypeOf" => factory.pure_fn_returns_boolean,
    "propertyIsEnumerable" => factory.pure_fn_returns_boolean,
    "toLocaleString" => factory.pure_fn_returns_string,
    "toString" => factory.pure_fn_returns_string,
    "valueOf" => factory.pure_fn_returns_unknown,
  })
}
