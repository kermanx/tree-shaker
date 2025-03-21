use crate::{
  analyzer::Analyzer, ast::AstKind2, consumable::ConsumableTrait, transformer::Transformer,
};
use oxc::span::{GetSpan, Span};
use rustc_hash::FxHashSet;
use std::{
  fmt::Debug,
  hash::Hash,
  sync::atomic::{AtomicUsize, Ordering},
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct DepId((usize, usize));

impl Debug for DepId {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    self.span().fmt(f)
  }
}

impl<'a> ConsumableTrait<'a> for DepId {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    analyzer.refer_dep(*self);
  }
}

impl<'a> From<AstKind2<'a>> for DepId {
  fn from(node: AstKind2<'a>) -> Self {
    DepId(unsafe { std::mem::transmute::<AstKind2<'_>, (usize, usize)>(node) })
  }
}

impl From<DepId> for AstKind2<'_> {
  fn from(val: DepId) -> Self {
    unsafe { std::mem::transmute(val.0) }
  }
}

impl GetSpan for DepId {
  fn span(&self) -> Span {
    let ast_kind: AstKind2<'_> = (*self).into();
    ast_kind.span()
  }
}

static COUNTER: AtomicUsize = AtomicUsize::new(0);

impl DepId {
  pub fn from_counter() -> Self {
    AstKind2::Index(COUNTER.fetch_add(1, Ordering::Relaxed)).into()
  }
}

#[derive(Default)]
pub struct ReferredDeps(FxHashSet<DepId>);

impl ReferredDeps {
  pub fn refer_dep(&mut self, dep: impl Into<DepId>) {
    self.0.insert(dep.into());
  }

  pub fn is_referred(&self, dep: impl Into<DepId>) -> bool {
    self.0.contains(&dep.into())
  }
}

impl Analyzer<'_> {
  pub fn refer_dep(&mut self, dep: impl Into<DepId>) {
    self.referred_deps.refer_dep(dep);
  }

  pub fn is_referred(&self, dep: impl Into<DepId>) -> bool {
    self.referred_deps.is_referred(dep)
  }
}

impl Transformer<'_> {
  pub fn is_referred(&self, dep: impl Into<DepId>) -> bool {
    self.referred_deps.is_referred(dep)
  }
}
