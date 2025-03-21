use crate::{
  analyzer::Analyzer,
  consumable::{Consumable, ConsumableTrait},
  entity::{Entity, EntityFactory, ObjectId, ObjectPrototype},
  init_object,
  scope::exhaustive::ExhaustiveDepId,
};
use oxc_index::{IndexVec, define_index_type};
use std::mem;

#[derive(Debug)]
pub struct ReactContextData<'a> {
  object_id: ObjectId,
  consumed: bool,
  default_value: Entity<'a>,
  stack: Vec<Entity<'a>>,
  dep: Consumable<'a>,
}

impl<'a> ReactContextData<'a> {
  pub fn get_current(&self, factory: &'a EntityFactory<'a>) -> Entity<'a> {
    factory.computed(
      if self.consumed {
        factory.unknown()
      } else {
        self.stack.last().copied().unwrap_or(self.default_value)
      },
      self.dep,
    )
  }
}

define_index_type! {
  pub struct ContextId = u32;
  DISABLE_MAX_INDEX_CHECK = cfg!(not(debug_assertions));
}

pub type ReactContexts<'a> = IndexVec<ContextId, ReactContextData<'a>>;

pub fn create_react_create_context_impl<'a>(factory: &'a EntityFactory<'a>) -> Entity<'a> {
  factory.implemented_builtin_fn("React::createContext", |analyzer, dep, _this, args| {
    let default_value =
      args.destruct_as_array(analyzer, analyzer.factory.empty_consumable, 1, false).0[0];

    let context = analyzer
      .new_empty_object(ObjectPrototype::Builtin(&analyzer.builtins.prototypes.object), None);

    let context_id = analyzer.builtins.react_data.contexts.push(ReactContextData {
      object_id: context.object_id,
      consumed: false,
      default_value,
      stack: Vec::new(),
      dep,
    });

    init_object!(context, factory, {
      "__#internal__consumed_hook" => analyzer.factory.computed_unknown(context_id),
      "__#internal__context_id" => analyzer.serialize_internal_id(context_id),
      "Provider" => create_react_context_provider_impl(analyzer, context_id),
      "Consumer" => create_react_context_consumer_impl(analyzer, context_id),
    });

    context.into()
  })
}

impl<'a> ConsumableTrait<'a> for ContextId {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    let data = &mut analyzer.builtins.react_data.contexts[*self];
    data.consumed = true;
    let default_value = data.default_value;
    let dep = data.dep;
    let stack = mem::take(&mut data.stack);
    analyzer.consume(default_value);
    analyzer.consume(dep);
    for value in stack {
      analyzer.consume(value);
    }
  }
}

fn create_react_context_provider_impl<'a>(
  analyzer: &mut Analyzer<'a>,
  context_id: ContextId,
) -> Entity<'a> {
  analyzer.dynamic_implemented_builtin(
    "React::Context::Provider",
    move |analyzer, dep, _this, args| {
      let props = args.destruct_as_array(analyzer, dep, 1, false).0[0];
      let value = props.get_property(analyzer, dep, analyzer.factory.string("value"));

      let data = &mut analyzer.builtins.react_data.contexts[context_id];
      let mut need_pop = false;
      if data.consumed {
        analyzer.consume(value);
      } else {
        data.stack.push(analyzer.factory.computed_unknown(value));

        let object_id = data.object_id;
        let dep = data.dep;

        let should_consume =
          analyzer.request_exhaustive_callbacks(ExhaustiveDepId::Object(object_id));

        if should_consume {
          analyzer.consume(context_id);
        } else {
          // Currently we can't remove <Context.Provider> from the tree,
          // so we need to consume the dep here.
          analyzer.consume(dep);

          let data = &mut analyzer.builtins.react_data.contexts[context_id];
          data.stack.pop();

          data.stack.push(value);
          need_pop = true;
        }
      }

      let children = props.get_property(analyzer, dep, analyzer.factory.string("children"));
      children.consume(analyzer);

      if need_pop {
        analyzer.builtins.react_data.contexts[context_id].stack.pop();
      }

      analyzer.factory.immutable_unknown
    },
  )
}

fn create_react_context_consumer_impl<'a>(
  analyzer: &mut Analyzer<'a>,
  context_id: ContextId,
) -> Entity<'a> {
  analyzer.dynamic_implemented_builtin(
    "React::Context::Consumer",
    move |analyzer, dep, _this, _args| {
      analyzer.consume(dep);
      analyzer.consume(context_id);

      analyzer.factory.immutable_unknown
    },
  )
}

pub fn create_react_use_context_impl<'a>(factory: &'a EntityFactory<'a>) -> Entity<'a> {
  factory.implemented_builtin_fn("React::useContext", move |analyzer, dep, _this, args| {
    let context_object =
      args.destruct_as_array(analyzer, analyzer.factory.empty_consumable, 1, false).0[0];
    let context_id = context_object.get_property(
      analyzer,
      analyzer.factory.empty_consumable,
      analyzer.factory.string("__#internal__context_id"),
    );
    if let Some(id) = analyzer.parse_internal_symbol_id::<ContextId>(context_id) {
      let data = &analyzer.builtins.react_data.contexts[id];
      factory.computed(data.get_current(factory), (context_id, dep))
    } else {
      factory.computed_unknown((context_object, dep))
    }
  })
}
