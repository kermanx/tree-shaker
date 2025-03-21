use super::{BuiltinPrototype, object::create_object_prototype};
use crate::{entity::EntityFactory, init_prototype};

pub fn create_regexp_prototype<'a>(factory: &EntityFactory<'a>) -> BuiltinPrototype<'a> {
  init_prototype!("RegExp", create_object_prototype(factory), {
    "exec" => factory.pure_fn_returns_unknown,
    "test" => factory.pure_fn_returns_boolean,
    "toString" => factory.pure_fn_returns_string,
  })
}
