mod class_names;
mod context;
mod create_element;
mod dependencies;
mod forward_ref;
mod jsx;
mod jsxs;
mod memo;
mod use_memo;

use super::{
  constants::{REACT_JSX_RUNTIME_NAMESPACE_OBJECT_ID, REACT_NAMESPACE_OBJECT_ID},
  prototypes::BuiltinPrototypes,
};
use crate::{
  entity::{Entity, EntityFactory, ObjectPropertyValue, ObjectPrototype},
  init_namespace,
};
pub use class_names::create_class_names_namespace;
use context::{ReactContexts, create_react_create_context_impl, create_react_use_context_impl};
use create_element::create_react_create_element_impl;
use dependencies::ReactDependencies;
use forward_ref::create_react_forward_ref_impl;
use jsx::create_react_jsx_impl;
use jsxs::create_react_jsxs_impl;
use memo::create_react_memo_impl;
use use_memo::{ReactUseMemos, create_react_use_memo_impl};

#[derive(Debug, Default)]
pub struct AnalyzerDataForReact<'a> {
  pub contexts: ReactContexts<'a>,
  pub memos: ReactUseMemos<'a>,
  pub dependencies: ReactDependencies<'a>,
  pub key_children: Option<Entity<'a>>,
}

pub fn create_react_namespace<'a>(
  factory: &'a EntityFactory<'a>,
  _prototypes: &'a BuiltinPrototypes<'a>,
) -> Entity<'a> {
  let namespace =
    factory.builtin_object(REACT_NAMESPACE_OBJECT_ID, ObjectPrototype::ImplicitOrNull, false);
  namespace.init_rest(factory, ObjectPropertyValue::Field(factory.immutable_unknown, true));

  init_namespace!(namespace, factory, {
    "forwardRef" => create_react_forward_ref_impl(factory),
    "memo" => create_react_memo_impl(factory),
    "createElement" => create_react_create_element_impl(factory),
    "createContext" => create_react_create_context_impl(factory),
    "useContext" => create_react_use_context_impl(factory),
    "useMemo" => create_react_use_memo_impl(factory),
  });

  namespace.into()
}

pub fn create_react_jsx_runtime_namespace<'a>(
  factory: &'a EntityFactory<'a>,
  _prototypes: &'a BuiltinPrototypes<'a>,
) -> Entity<'a> {
  let object = factory.builtin_object(
    REACT_JSX_RUNTIME_NAMESPACE_OBJECT_ID,
    ObjectPrototype::ImplicitOrNull,
    false,
  );
  object.init_rest(factory, ObjectPropertyValue::Field(factory.immutable_unknown, true));

  init_namespace!(object, factory, {
    "jsx" => create_react_jsx_impl(factory),
    "jsxs" => create_react_jsxs_impl(factory),
  });

  object.into()
}
