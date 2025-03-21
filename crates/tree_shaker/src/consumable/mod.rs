mod collector;
mod impls;
mod lazy;
mod once;

use crate::analyzer::Analyzer;
pub use collector::*;
pub use lazy::*;
pub use once::*;
use oxc::allocator::{self, Allocator};
use std::fmt::Debug;

pub trait ConsumableTrait<'a>: Debug {
  fn consume(&self, analyzer: &mut Analyzer<'a>);
}

pub trait ConsumeTrait<'a>: Debug {
  fn consume(&self, analyzer: &mut Analyzer<'a>);
  fn uniform(self, allocator: &'a Allocator) -> Consumable<'a>;
}

impl<'a, T: ConsumableTrait<'a> + 'a> ConsumeTrait<'a> for T {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    self.consume(analyzer);
  }
  fn uniform(self, allocator: &'a Allocator) -> Consumable<'a> {
    Consumable(allocator.alloc(self))
  }
}

#[derive(Debug, Clone, Copy)]
pub struct Consumable<'a>(pub &'a (dyn ConsumableTrait<'a> + 'a));

impl<'a> ConsumeTrait<'a> for Consumable<'a> {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    self.0.consume(analyzer);
  }
  fn uniform(self, _: &'a Allocator) -> Consumable<'a> {
    self
  }
}

pub type ConsumableVec<'a> = allocator::Vec<'a, Consumable<'a>>;

impl<'a> Analyzer<'a> {
  #[inline]
  pub fn consume(&mut self, dep: impl ConsumeTrait<'a> + 'a) {
    dep.consume(self);
  }

  pub fn consumable(&self, dep: impl ConsumableTrait<'a> + 'a) -> Consumable<'a> {
    self.factory.consumable(dep)
  }
}
