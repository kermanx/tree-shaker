use crate::transformer::Transformer;

impl<'a> Transformer<'a> {
  pub fn escape_template_element_value(&self, value: &str) -> &'a str {
    let mut result = String::new();
    for c in value.chars() {
      match c {
        '\\' => result.push_str("\\\\"),
        '`' => result.push_str("\\`"),
        '$' => result.push_str("\\$"),
        _ => result.push(c),
      }
    }
    self.allocator.alloc_str(&result)
  }
}
