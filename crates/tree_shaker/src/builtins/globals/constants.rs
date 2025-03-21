use crate::{builtins::Builtins, init_map};

impl Builtins<'_> {
  pub fn init_global_constants(&mut self) {
    let factory = self.factory;

    init_map!(self.globals, {
      "undefined" => factory.undefined,
      "Infinity" => factory.infinity(true),
      "NaN" => factory.nan,
      "eval" => factory.immutable_unknown,
    })
  }
}
