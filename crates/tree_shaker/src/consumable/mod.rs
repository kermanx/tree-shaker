mod collector;
mod impls;
mod lazy;
mod once;

use crate::analyzer::Analyzer;
pub use collector::*;
pub use lazy::*;
pub use once::*;
use std::fmt::Debug;

pub trait ConsumableTrait<'a>: Debug {
  fn consume(&self, analyzer: &mut Analyzer<'a>);
}

#[derive(Debug, Clone, Copy)]
pub struct Consumable<'a>(pub &'a (dyn ConsumableTrait<'a> + 'a));

pub type ConsumableVec<'a> = Vec<Consumable<'a>>;

impl<'a> Analyzer<'a> {
  #[inline]
  pub fn consume(&mut self, dep: impl ConsumableTrait<'a> + 'a) {
    dep.consume(self);
  }

  pub fn consumable(&self, dep: impl ConsumableTrait<'a> + 'a) -> Consumable<'a> {
    self.factory.consumable(dep)
  }
}
