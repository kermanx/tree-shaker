use super::{Consumable, ConsumableTrait, ConsumableVec};
use crate::analyzer::Analyzer;
use std::cell::RefCell;

#[derive(Debug, Clone, Copy)]
pub struct LazyConsumable<'a>(pub &'a RefCell<Option<ConsumableVec<'a>>>);

impl<'a> ConsumableTrait<'a> for LazyConsumable<'a> {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    self.0.take().consume(analyzer);
  }
}

impl<'a> LazyConsumable<'a> {
  pub fn push(&self, analyzer: &mut Analyzer<'a>, consumable: Consumable<'a>) {
    let mut consumables_ref = self.0.borrow_mut();
    if let Some(consumables) = consumables_ref.as_mut() {
      consumables.push(consumable);
    } else {
      drop(consumables_ref);
      analyzer.consume(consumable);
    }
  }
}
