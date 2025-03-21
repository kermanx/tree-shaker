use crate::entity::EntityFactory;

use super::{BuiltinPrototype, object::create_object_prototype};

pub fn create_bigint_prototype<'a>(factory: &EntityFactory<'a>) -> BuiltinPrototype<'a> {
  create_object_prototype(factory).with_name("BigInt")
}
