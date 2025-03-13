use crate::entity::EntityFactory;

use super::{object::create_object_prototype, BuiltinPrototype};

pub fn create_bigint_prototype<'a>(factory: &EntityFactory<'a>) -> BuiltinPrototype<'a> {
  create_object_prototype(factory).with_name("BigInt")
}
