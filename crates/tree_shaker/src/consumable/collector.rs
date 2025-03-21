use super::{Consumable, ConsumeTrait};
use crate::{analyzer::Analyzer, entity::EntityFactory};
use oxc::allocator;
use std::mem;

#[derive(Debug)]
pub struct ConsumableCollector<'a, T: ConsumeTrait<'a> + 'a = Consumable<'a>> {
  pub current: allocator::Vec<'a, T>,
  pub node: Option<Consumable<'a>>,
}

impl<'a, T: ConsumeTrait<'a> + 'a> ConsumableCollector<'a, T> {
  pub fn new(current: allocator::Vec<'a, T>) -> Self {
    Self { current, node: None }
  }

  pub fn is_empty(&self) -> bool {
    self.current.is_empty() && self.node.is_none()
  }

  pub fn push(&mut self, value: T) {
    self.current.push(value);
  }

  pub fn try_collect(&mut self, factory: &EntityFactory<'a>) -> Option<Consumable<'a>> {
    if self.current.is_empty() {
      self.node
    } else {
      let current = mem::replace(&mut self.current, factory.vec());
      let node = Some(if let Some(node) = self.node {
        factory.consumable((current, node))
      } else {
        factory.consumable(current)
      });
      self.node = node;
      node
    }
  }

  pub fn collect(&mut self, factory: &EntityFactory<'a>) -> Consumable<'a> {
    self.try_collect(factory).unwrap_or(factory.empty_consumable)
  }

  pub fn consume_all(&self, analyzer: &mut Analyzer<'a>) {
    for value in &self.current {
      value.consume(analyzer);
    }

    if let Some(node) = self.node {
      node.consume(analyzer);
    }
  }

  pub fn force_clear(&mut self) {
    self.current.clear();
    self.node = None;
  }

  pub fn may_not_referred(&self) -> bool {
    !self.current.is_empty() || self.node.is_some()
  }
}
