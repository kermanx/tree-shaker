use super::{ConsumableTrait, ConsumeTrait};
use crate::analyzer::Analyzer;
use oxc::allocator;
use std::cell::RefCell;

impl<'a> ConsumableTrait<'a> for () {
  fn consume(&self, _: &mut Analyzer<'a>) {}
}

impl<'a, T: ConsumeTrait<'a> + 'a> ConsumableTrait<'a> for allocator::Box<'a, T> {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    self.as_ref().consume(analyzer)
  }
}

impl<'a, T: ConsumeTrait<'a> + 'a> ConsumableTrait<'a> for Option<T> {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    if let Some(value) = self {
      value.consume(analyzer)
    }
  }
}

impl<'a, T: ConsumeTrait<'a> + 'a> ConsumableTrait<'a> for &'a RefCell<T> {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    self.borrow().consume(analyzer)
  }
}

impl<'a, T: ConsumeTrait<'a> + 'a> ConsumableTrait<'a> for allocator::Vec<'a, T> {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    for item in self {
      item.consume(analyzer)
    }
  }
}

impl<'a, T: ConsumeTrait<'a> + 'a> ConsumableTrait<'a> for &allocator::Vec<'a, T> {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    for item in *self {
      item.consume(analyzer)
    }
  }
}

impl<'a, T1: ConsumeTrait<'a> + 'a, T2: ConsumeTrait<'a> + 'a> ConsumableTrait<'a> for (T1, T2) {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    self.0.consume(analyzer);
    self.1.consume(analyzer)
  }
}

impl<'a, T1: ConsumeTrait<'a> + 'a, T2: ConsumeTrait<'a> + 'a, T3: ConsumeTrait<'a> + 'a>
  ConsumableTrait<'a> for (T1, T2, T3)
{
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    self.0.consume(analyzer);
    self.1.consume(analyzer);
    self.2.consume(analyzer);
  }
}

impl<
  'a,
  T1: ConsumeTrait<'a> + 'a,
  T2: ConsumeTrait<'a> + 'a,
  T3: ConsumeTrait<'a> + 'a,
  T4: ConsumeTrait<'a> + 'a,
> ConsumableTrait<'a> for (T1, T2, T3, T4)
{
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    self.0.consume(analyzer);
    self.1.consume(analyzer);
    self.2.consume(analyzer);
    self.3.consume(analyzer);
  }
}
