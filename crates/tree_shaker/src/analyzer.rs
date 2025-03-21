use crate::{
  TreeShakeConfig,
  builtins::Builtins,
  dep::{DepId, ReferredDeps},
  entity::EntityFactory,
  folding::ConstantFolder,
  mangling::Mangler,
  module::{ModuleId, Modules},
  scope::{
    Scoping,
    conditional::ConditionalDataMap,
    exhaustive::{ExhaustiveCallback, ExhaustiveDepId},
    // r#loop::LoopDataMap,
  },
  utils::ExtraData,
  vfs::Vfs,
};
use oxc::{
  allocator::Allocator,
  span::{GetSpan, Span},
};
use rustc_hash::{FxHashMap, FxHashSet};
use std::{collections::BTreeSet, mem};

pub struct Analyzer<'a> {
  pub vfs: Box<dyn Vfs>,
  pub config: &'a TreeShakeConfig,
  pub allocator: &'a Allocator,
  pub factory: &'a EntityFactory<'a>,

  pub modules: Modules<'a>,
  pub builtins: Builtins<'a>,

  pub module_stack: Vec<ModuleId>,
  pub span_stack: Vec<Span>,
  pub scoping: Scoping<'a>,

  pub data: ExtraData<'a>,
  pub exhaustive_callbacks: FxHashMap<ExhaustiveDepId, FxHashSet<ExhaustiveCallback<'a>>>,
  pub referred_deps: ReferredDeps,
  pub conditional_data: ConditionalDataMap<'a>,
  // pub loop_data: LoopDataMap<'a>,
  pub folder: ConstantFolder<'a>,
  pub mangler: Mangler<'a>,
  pub pending_deps: FxHashSet<ExhaustiveCallback<'a>>,
  pub diagnostics: BTreeSet<String>,
}

impl<'a> Analyzer<'a> {
  pub fn new_in(vfs: Box<dyn Vfs>, config: &'a TreeShakeConfig, allocator: &'a Allocator) -> Self {
    let factory = &*allocator.alloc(EntityFactory::new(allocator, config));
    Analyzer {
      vfs,
      config,
      allocator,
      factory,

      modules: Modules::default(),
      builtins: Builtins::new(config, factory),

      module_stack: vec![],
      span_stack: vec![],
      scoping: Scoping::new(factory),

      data: Default::default(),
      exhaustive_callbacks: Default::default(),
      referred_deps: Default::default(),
      conditional_data: Default::default(),
      // loop_data: Default::default(),
      folder: Default::default(),
      mangler: Mangler::new(config.mangling.is_some(), allocator),
      pending_deps: Default::default(),
      diagnostics: Default::default(),
    }
  }

  pub fn finalize(&mut self) {
    self.module_stack.push(ModuleId::new(0));

    self.consume_exports(ModuleId::new(0));

    let mut round = 0usize;
    loop {
      round += 1;
      if round > 1000 {
        panic!("Possible infinite loop in post analysis");
      }

      let mut dirty = false;
      dirty |= self.consume_top_level_uncaught();
      dirty |= self.call_exhaustive_callbacks();
      dirty |= self.post_analyze_handle_conditional();
      // dirty |= self.post_analyze_handle_loops();
      dirty |= self.post_analyze_handle_folding();
      if !dirty {
        break;
      }
    }

    self.module_stack.pop();

    #[cfg(feature = "flame")]
    flamescope::dump(&mut std::fs::File::create("flamescope.json").unwrap()).unwrap();
  }

  fn consume_top_level_uncaught(&mut self) -> bool {
    let factory = self.factory;
    let thrown_values = &mut self.call_scope_mut().try_scopes.last_mut().unwrap().thrown_values;
    if thrown_values.is_empty() {
      false
    } else {
      let values = mem::replace(thrown_values, factory.vec());
      self.consume(values);
      true
    }
  }
}

impl<'a> Analyzer<'a> {
  pub fn set_data(&mut self, key: impl Into<DepId>, data: impl Default + 'a) {
    self.data.insert(key.into(), unsafe { mem::transmute(Box::new(data)) });
  }

  pub fn get_data_or_insert_with<D: 'a>(
    &mut self,
    key: impl Into<DepId>,
    default: impl FnOnce() -> D,
  ) -> &'a mut D {
    let boxed =
      self.data.entry(key.into()).or_insert_with(|| unsafe { mem::transmute(Box::new(default())) });
    unsafe { mem::transmute(boxed.as_mut()) }
  }

  pub fn load_data<D: Default + 'a>(&mut self, key: impl Into<DepId>) -> &'a mut D {
    self.get_data_or_insert_with(key, Default::default)
  }

  pub fn add_diagnostic(&mut self, message: impl Into<String>) {
    if let Some(span) = self.span_stack.last() {
      let start = self.line_index().line_col(span.start.into());
      let end = self.line_index().line_col(span.end.into());
      let span_text =
        format!(" at {}:{}-{}:{}", start.line + 1, start.col + 1, end.line + 1, end.col + 1);
      self.diagnostics.insert(message.into() + &span_text);
    } else {
      self.diagnostics.insert(message.into());
    }
  }

  pub fn current_module(&self) -> ModuleId {
    *self.module_stack.last().unwrap()
  }

  pub fn current_span(&self) -> Span {
    *self.span_stack.last().unwrap()
  }

  pub fn push_span(&mut self, node: &impl GetSpan) {
    self.span_stack.push(node.span());
  }

  pub fn pop_span(&mut self) {
    self.span_stack.pop();
  }
}

impl<'a> From<Analyzer<'a>> for &'a Allocator {
  fn from(val: Analyzer<'a>) -> Self {
    val.allocator
  }
}
