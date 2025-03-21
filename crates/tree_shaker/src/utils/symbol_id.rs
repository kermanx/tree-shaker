use crate::{
  analyzer::Analyzer,
  entity::{Entity, LiteralEntity},
};
use oxc_index::Idx;

impl<'a> Analyzer<'a> {
  pub fn serialize_internal_id(&self, symbol_id: impl Idx) -> Entity<'a> {
    self.factory.string(self.allocator.alloc_str(&format!("__#symbol__{}", symbol_id.index())))
  }

  pub fn parse_internal_symbol_id<T: Idx>(&self, entity: Entity<'a>) -> Option<T> {
    let literal = entity.get_literal(self)?;
    let LiteralEntity::String(string, _) = literal else { return None };
    if let Some(id) = string.strip_prefix("__#symbol__") {
      id.parse().ok().map(Idx::from_usize)
    } else {
      None
    }
  }
}
