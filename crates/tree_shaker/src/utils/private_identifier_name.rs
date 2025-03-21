use crate::analyzer::Analyzer;

impl<'a> Analyzer<'a> {
  pub fn escape_private_identifier_name(&self, name: &str) -> &'a str {
    self.allocator.alloc_str(&format!("__#private__{}", name))
  }
}
