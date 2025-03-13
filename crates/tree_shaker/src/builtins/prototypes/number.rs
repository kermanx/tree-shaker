use super::{object::create_object_prototype, BuiltinPrototype};
use crate::{entity::EntityFactory, init_prototype};

pub fn create_number_prototype<'a>(factory: &EntityFactory<'a>) -> BuiltinPrototype<'a> {
  init_prototype!("Number", create_object_prototype(factory), {
    "toExponential" => factory.pure_fn_returns_string,
    "toFixed" => factory.pure_fn_returns_string,
    "toLocaleString" => factory.pure_fn_returns_string,
    "toPrecision" => factory.pure_fn_returns_string,
    "valueOf" => factory.pure_fn_returns_number,
  })
}
