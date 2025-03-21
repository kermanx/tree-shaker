use crate::{
  analyzer::Analyzer,
  consumable::{Consumable, ConsumableCollector},
  entity::Entity,
  module::ModuleId,
  scope::CfScopeKind,
};
use oxc::span::Span;
use rustc_hash::FxHashMap;
use std::{cell::RefCell, rc::Rc, vec};

#[derive(Debug)]
pub struct ReactDependenciesData<'a> {
  pub collectors: Vec<ConsumableCollector<'a, Entity<'a>>>,
  pub rest_collector: ConsumableCollector<'a, Entity<'a>>,
  pub extra_collector: ConsumableCollector<'a>,
  /// `None`: initial
  /// `vec![None]`: has changed
  /// `vec![Some(value)]`: always be value
  pub previous: Option<Vec<Option<Entity<'a>>>>,
}

pub type ReactDependencies<'a> =
  FxHashMap<(ModuleId, Span), Rc<RefCell<ReactDependenciesData<'a>>>>;

/// Returns (is_changed, consumable)
pub fn check_dependencies<'a>(
  analyzer: &mut Analyzer<'a>,
  dep: Consumable<'a>,
  current: Entity<'a>,
) -> (bool, Consumable<'a>) {
  let factory = analyzer.factory;
  let (elements, rest, iterate_dep) = current.iterate(analyzer, dep);

  let span = (analyzer.current_module(), analyzer.current_span());
  let data = analyzer
    .builtins
    .react_data
    .dependencies
    .entry(span)
    .or_insert_with(|| {
      Rc::new(RefCell::new(ReactDependenciesData {
        collectors: vec![],
        rest_collector: ConsumableCollector::new(factory.vec()),
        extra_collector: ConsumableCollector::new(factory.vec()),
        previous: None,
      }))
    })
    .clone();
  let mut data = data.borrow_mut();

  if data.collectors.len() <= elements.len() {
    data.collectors.resize_with(elements.len(), || ConsumableCollector::new(factory.vec()));
  }
  for (index, element) in elements.iter().enumerate() {
    data.collectors[index].push(*element);
  }

  let ReactDependenciesData { collectors, rest_collector, extra_collector, previous } = &mut *data;
  extra_collector.push(iterate_dep);

  let mut require_rerun = !rest_collector.is_empty() || rest.is_some();
  let result = if let Some(previous) = previous {
    let mut changed = vec![];
    for (index, element) in elements.iter().enumerate() {
      match previous.get(index) {
        Some(Some(old)) => {
          if analyzer.op_strict_eq(*element, *old).0 != Some(true) {
            changed.push(index);
            require_rerun = true;
          }
        }
        Some(None) => {
          changed.push(index);
        }
        None => {
          changed.push(index);
          require_rerun = true;
        }
      }
    }
    for (index, _) in previous.iter().enumerate().skip(elements.len()) {
      // shorter than previous
      changed.push(index);
      require_rerun = true;
    }

    if let Some(rest) = rest {
      rest_collector.push(rest);
    }

    if changed.is_empty() {
      if let Some(rest) = rest_collector.try_collect(factory) {
        (true, analyzer.consumable((rest, extra_collector.collect(factory))))
      } else {
        (false, extra_collector.collect(factory))
      }
    } else {
      let mut deps = analyzer.factory.vec();
      for index in &changed {
        deps.push(collectors[*index].collect(factory));
        if let Some(previous) = previous.get_mut(*index) {
          *previous = None;
        }
      }
      let last_changed = *changed.last().unwrap();
      if last_changed >= previous.len() {
        previous.resize(last_changed + 1, None);
      }
      (
        true,
        analyzer.consumable((
          deps,
          rest_collector.collect(factory),
          extra_collector.collect(factory),
        )),
      )
    }
  } else {
    require_rerun = true;
    for element in &elements {
      collectors.push(ConsumableCollector::new(analyzer.factory.vec1(*element)));
    }
    if let Some(rest) = rest {
      rest_collector.push(rest);
    }
    *previous = Some(elements.into_iter().map(Option::Some).collect());
    (true, analyzer.consumable((rest, extra_collector.collect(factory))))
  };

  if require_rerun {
    for depth in 0..analyzer.scoping.cf.stack.len() {
      if let CfScopeKind::Exhaustive(exhaustive_data) =
        &mut analyzer.scoping.cf.get_mut_from_depth(depth).kind
      {
        exhaustive_data.clean = false;
        break;
      }
    }
  }

  result
}
