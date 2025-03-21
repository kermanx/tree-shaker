use super::{
  Entity, EntityFactory, EnumeratedProperties, IteratedElements, ObjectEntity, ObjectPrototype,
  TypeofResult, ValueTrait, consumed_object, never::NeverEntity,
};
use crate::{analyzer::Analyzer, consumable::Consumable};
use std::fmt::Debug;

trait BuiltinFnEntity<'a>: Debug {
  #[cfg(feature = "flame")]
  fn name(&self) -> &'static str;
  fn object(&self) -> Option<&'a ObjectEntity<'a>> {
    None
  }
  fn call_impl(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    this: Entity<'a>,
    args: Entity<'a>,
  ) -> Entity<'a>;
}

impl<'a, T: BuiltinFnEntity<'a>> ValueTrait<'a> for T {
  fn consume(&'a self, analyzer: &mut Analyzer<'a>) {
    if let Some(object) = self.object() {
      object.consume(analyzer);
    }
  }

  fn unknown_mutate(&'a self, _analyzer: &mut Analyzer<'a>, _dep: Consumable<'a>) {
    // No effect
  }

  fn get_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    key: Entity<'a>,
  ) -> Entity<'a> {
    if let Some(object) = self.object() {
      object.get_property(analyzer, dep, key)
    } else {
      analyzer.builtins.prototypes.function.get_property(analyzer, self.into(), key, dep)
    }
  }

  fn set_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    key: Entity<'a>,
    value: Entity<'a>,
  ) {
    if let Some(object) = self.object() {
      object.set_property(analyzer, dep, key, value)
    } else {
      analyzer.add_diagnostic(
      "Should not set property of builtin function, it may cause unexpected tree-shaking behavior",
    );
      consumed_object::set_property(analyzer, dep, key, value)
    }
  }

  fn delete_property(&'a self, analyzer: &mut Analyzer<'a>, dep: Consumable<'a>, key: Entity<'a>) {
    if let Some(object) = self.object() {
      object.delete_property(analyzer, dep, key)
    } else {
      analyzer.add_diagnostic("Should not delete property of builtin function, it may cause unexpected tree-shaking behavior");
      consumed_object::delete_property(analyzer, dep, key)
    }
  }

  fn enumerate_properties(
    &'a self,
    _analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
  ) -> EnumeratedProperties<'a> {
    (vec![], dep)
  }

  fn call(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    this: Entity<'a>,
    args: Entity<'a>,
  ) -> Entity<'a> {
    #[cfg(feature = "flame")]
    let _scope_guard = flame::start_guard(self.name());
    self.call_impl(analyzer, dep, this, args)
  }

  fn construct(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    args: Entity<'a>,
  ) -> Entity<'a> {
    consumed_object::construct(self, analyzer, dep, args)
  }

  fn jsx(&'a self, analyzer: &mut Analyzer<'a>, props: Entity<'a>) -> Entity<'a> {
    self.call_impl(
      analyzer,
      analyzer.factory.empty_consumable,
      analyzer.factory.immutable_unknown,
      analyzer.factory.arguments(analyzer.factory.vec1((false, props))),
    )
  }

  fn r#await(&'a self, _analyzer: &mut Analyzer<'a>, _dep: Consumable<'a>) -> Entity<'a> {
    self.into()
  }

  fn iterate(&'a self, analyzer: &mut Analyzer<'a>, dep: Consumable<'a>) -> IteratedElements<'a> {
    analyzer.throw_builtin_error("Cannot iterate over function");
    if analyzer.config.preserve_exceptions {
      consumed_object::iterate(analyzer, dep)
    } else {
      NeverEntity.iterate(analyzer, dep)
    }
  }

  fn get_typeof(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.string("function")
  }

  fn get_to_string(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.computed_unknown_string(self)
  }

  fn get_to_numeric(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.nan
  }

  fn get_to_boolean(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.boolean(true)
  }

  fn get_to_property_key(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.get_to_string(analyzer)
  }

  fn get_to_jsx_child(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    // TODO: analyzer.thrown_builtin_error("Functions are not valid JSX children");
    analyzer.factory.string("")
  }

  fn get_constructor_prototype(
    &'a self,
    _analyzer: &Analyzer<'a>,
    _dep: Consumable<'a>,
  ) -> Option<(Consumable<'a>, ObjectPrototype<'a>, ObjectPrototype<'a>)> {
    todo!()
  }

  fn test_typeof(&self) -> TypeofResult {
    TypeofResult::Function
  }

  fn test_truthy(&self) -> Option<bool> {
    Some(true)
  }

  fn test_nullish(&self) -> Option<bool> {
    Some(false)
  }
}

pub trait BuiltinFnImplementation<'a>:
  Fn(&mut Analyzer<'a>, Consumable<'a>, Entity<'a>, Entity<'a>) -> Entity<'a>
{
}
impl<'a, T: Fn(&mut Analyzer<'a>, Consumable<'a>, Entity<'a>, Entity<'a>) -> Entity<'a>>
  BuiltinFnImplementation<'a> for T
{
}

#[derive(Clone, Copy)]
pub struct ImplementedBuiltinFnEntity<'a, F: BuiltinFnImplementation<'a> + 'a> {
  #[cfg(feature = "flame")]
  pub name: &'static str,
  pub implementation: F,
  pub object: Option<&'a ObjectEntity<'a>>,
}

impl<'a, F: BuiltinFnImplementation<'a> + 'a> Debug for ImplementedBuiltinFnEntity<'a, F> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("ImplementedBuiltinFnEntity").finish()
  }
}

impl<'a, F: BuiltinFnImplementation<'a> + 'a> BuiltinFnEntity<'a>
  for ImplementedBuiltinFnEntity<'a, F>
{
  #[cfg(feature = "flame")]
  fn name(&self) -> &'static str {
    self.name
  }
  fn object(&self) -> Option<&'a ObjectEntity<'a>> {
    self.object
  }
  fn call_impl(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    this: Entity<'a>,
    args: Entity<'a>,
  ) -> Entity<'a> {
    (self.implementation)(analyzer, dep, this, args)
  }
}

impl<'a> Analyzer<'a> {
  pub fn dynamic_implemented_builtin<F: BuiltinFnImplementation<'a> + 'a>(
    &mut self,
    _name: &'static str,
    implementation: F,
  ) -> Entity<'a> {
    self
      .factory
      .alloc(ImplementedBuiltinFnEntity {
        #[cfg(feature = "flame")]
        name: _name,
        implementation,
        object: Some(self.new_function_object(None).0),
      })
      .into()
  }
}

#[derive(Debug, Clone)]
pub struct PureBuiltinFnEntity<'a> {
  return_value: fn(&EntityFactory<'a>) -> Entity<'a>,
}

impl<'a> BuiltinFnEntity<'a> for PureBuiltinFnEntity<'a> {
  #[cfg(feature = "flame")]
  fn name(&self) -> &'static str {
    "<PureBuiltin>"
  }
  fn call_impl(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    this: Entity<'a>,
    args: Entity<'a>,
  ) -> Entity<'a> {
    let ret_val = (self.return_value)(analyzer.factory);
    let dep = analyzer.consumable((dep, this, args));
    this.unknown_mutate(analyzer, dep);
    args.unknown_mutate(analyzer, dep);
    analyzer.factory.computed(ret_val, dep)
  }
}

impl<'a> PureBuiltinFnEntity<'a> {
  pub fn new(return_value: fn(&EntityFactory<'a>) -> Entity<'a>) -> Self {
    Self { return_value }
  }
}
