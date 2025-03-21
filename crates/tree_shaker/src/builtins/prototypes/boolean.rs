use super::{BuiltinPrototype, object::create_object_prototype};
use crate::{entity::EntityFactory, init_prototype};

pub fn create_boolean_prototype<'a>(factory: &EntityFactory<'a>) -> BuiltinPrototype<'a> {
  init_prototype!("Boolean", create_object_prototype(factory), {
    "valueOf" => factory.pure_fn_returns_boolean,
  })
}
