use super::BuiltinPrototype;
use crate::entity::EntityFactory;

pub fn create_null_prototype<'a>(_factory: &EntityFactory<'a>) -> BuiltinPrototype<'a> {
  BuiltinPrototype::default().with_name("null")
}
