use super::BuiltinPrototype;
use crate::entity::EntityFactory;

pub fn create_null_prototype<'a>(factory: &EntityFactory<'a>) -> BuiltinPrototype<'a> {
  BuiltinPrototype::new_in(factory).with_name("null")
}
