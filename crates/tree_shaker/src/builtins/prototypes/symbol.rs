use super::{BuiltinPrototype, object::create_object_prototype};
use crate::{entity::EntityFactory, init_prototype};

pub fn create_symbol_prototype<'a>(factory: &EntityFactory<'a>) -> BuiltinPrototype<'a> {
  init_prototype!("Symbol", create_object_prototype(factory), {
    "toString" => factory.pure_fn_returns_string,
    "valueOf" => factory.pure_fn_returns_symbol,
    "description" => factory.pure_fn_returns_string,
  })
}
