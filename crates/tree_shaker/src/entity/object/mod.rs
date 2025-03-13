mod delete;
mod enumerate;
mod get;
mod init;
mod property;
mod set;

use super::{
  consumed_object, Entity, EntityTrait, EnumeratedProperties, IteratedElements, LiteralEntity,
  TypeofResult,
};
use crate::{
  analyzer::Analyzer,
  builtins::BuiltinPrototype,
  consumable::{Consumable, ConsumableTrait},
  dep::DepId,
  mangling::{is_literal_mangable, MangleAtom, UniquenessGroupId},
  scope::CfScopeId,
  use_consumed_flag,
  utils::ast::AstKind2,
};
use oxc::semantic::SymbolId;
pub use property::{ObjectProperty, ObjectPropertyValue};
use rustc_hash::{FxHashMap, FxHashSet};
use std::cell::{Cell, RefCell};

type ObjectManglingGroupId<'a> = &'a Cell<Option<UniquenessGroupId>>;

#[derive(Debug, Clone, Copy)]
pub enum ObjectPrototype<'a> {
  ImplicitOrNull,
  Builtin(&'a BuiltinPrototype<'a>),
  Custom(&'a ObjectEntity<'a>),
  Unknown(Consumable<'a>),
}

impl<'a> ConsumableTrait<'a> for ObjectPrototype<'a> {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    match self {
      ObjectPrototype::ImplicitOrNull => {}
      ObjectPrototype::Builtin(_prototype) => {}
      ObjectPrototype::Custom(object) => object.consume_as_prototype(analyzer),
      ObjectPrototype::Unknown(dep) => dep.consume(analyzer),
    }
  }
}

#[derive(Debug)]
pub struct ObjectEntity<'a> {
  /// A built-in object is usually non-consumable
  pub consumable: bool,
  pub consumed: Cell<bool>,
  pub consumed_as_prototype: Cell<bool>,
  // deps: RefCell<ConsumableCollector<'a>>,
  /// Where the object is created
  pub cf_scope: CfScopeId,
  pub object_id: SymbolId,
  pub prototype: Cell<ObjectPrototype<'a>>,
  /// `None` if not mangable
  /// `Some(None)` if mangable at the beginning, but disabled later
  pub mangling_group: Option<ObjectManglingGroupId<'a>>,

  /// Properties keyed by known string
  pub string_keyed: RefCell<FxHashMap<&'a str, ObjectProperty<'a>>>,
  /// Properties keyed by unknown value
  pub unknown_keyed: RefCell<ObjectProperty<'a>>,
  /// Properties keyed by unknown value, but not included in `string_keyed`
  pub rest: RefCell<Option<ObjectProperty<'a>>>,
  // TODO: symbol_keyed
}

impl<'a> EntityTrait<'a> for ObjectEntity<'a> {
  fn consume(&'a self, analyzer: &mut Analyzer<'a>) {
    if !self.consumable {
      return;
    }

    use_consumed_flag!(self);

    self.consume_as_prototype(analyzer);

    self.string_keyed.borrow_mut().clear();
    self.unknown_keyed.take();

    analyzer.mark_object_consumed(self.cf_scope, self.object_id);
  }

  fn unknown_mutate(&'a self, analyzer: &mut Analyzer<'a>, dep: Consumable<'a>) {
    if self.consumed.get() {
      return consumed_object::unknown_mutate(analyzer, dep);
    }

    self.unknown_keyed.borrow_mut().non_existent.push(dep);
  }

  fn get_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    key: Entity<'a>,
  ) -> Entity<'a> {
    self.get_property(analyzer, dep, key)
  }

  fn set_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    key: Entity<'a>,
    value: Entity<'a>,
  ) {
    self.set_property(analyzer, dep, key, value);
  }

  fn enumerate_properties(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
  ) -> EnumeratedProperties<'a> {
    self.enumerate_properties(analyzer, dep)
  }

  fn delete_property(&'a self, analyzer: &mut Analyzer<'a>, dep: Consumable<'a>, key: Entity<'a>) {
    self.delete_property(analyzer, dep, key);
  }

  fn call(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    this: Entity<'a>,
    args: Entity<'a>,
  ) -> Entity<'a> {
    consumed_object::call(self, analyzer, dep, this, args)
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
    consumed_object::jsx(self, analyzer, props)
  }

  fn r#await(&'a self, analyzer: &mut Analyzer<'a>, dep: Consumable<'a>) -> Entity<'a> {
    self.consume(analyzer);
    consumed_object::r#await(analyzer, dep)
  }

  fn iterate(&'a self, analyzer: &mut Analyzer<'a>, dep: Consumable<'a>) -> IteratedElements<'a> {
    self.consume(analyzer);
    consumed_object::iterate(analyzer, dep)
  }

  fn get_destructable(&'a self, _analyzer: &Analyzer<'a>, dep: Consumable<'a>) -> Consumable<'a> {
    dep
  }

  fn get_typeof(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.string("object")
  }

  fn get_to_string(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    // FIXME: Special methods
    if self.consumed.get() {
      return consumed_object::get_to_string(analyzer);
    }
    analyzer.factory.computed_unknown_string(self)
  }

  fn get_to_numeric(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    // FIXME: Special methods
    if self.consumed.get() {
      return consumed_object::get_to_numeric(analyzer);
    }
    analyzer.factory.computed_unknown(self)
  }

  fn get_to_boolean(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.boolean(true)
  }

  fn get_to_property_key(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.get_to_string(analyzer)
  }

  fn get_to_jsx_child(&'a self, _analyzer: &Analyzer<'a>) -> Entity<'a> {
    self
  }

  fn get_own_keys(&'a self, analyzer: &Analyzer<'a>) -> Option<Vec<(bool, Entity<'a>)>> {
    if self.consumed.get()
      || self.rest.borrow().is_some()
      || !self.unknown_keyed.borrow().possible_values.is_empty()
    {
      return None;
    }

    let mut keys = Vec::new();
    for (key, property) in self.string_keyed.borrow_mut().iter_mut() {
      let key_entity = property.key.unwrap_or_else(|| analyzer.factory.string(key));
      let key_entity = if property.non_existent.is_empty() {
        key_entity
      } else {
        analyzer.factory.computed(key_entity, property.non_existent.collect(analyzer.factory))
      };
      let key_entity = analyzer.factory.computed(
        key_entity,
        analyzer.factory.consumable(
          property
            .possible_values
            .iter()
            .map(|value| match value {
              ObjectPropertyValue::Field(value, _) => *value,
              ObjectPropertyValue::Property(Some(getter), _) => *getter,
              ObjectPropertyValue::Property(None, _) => analyzer.factory.undefined,
            })
            .collect::<Vec<_>>(),
        ),
      );
      keys.push((property.definite, key_entity));
    }
    Some(keys)
  }

  fn test_typeof(&self) -> TypeofResult {
    TypeofResult::Object
  }

  fn test_truthy(&self) -> Option<bool> {
    Some(true)
  }

  fn test_nullish(&self) -> Option<bool> {
    Some(false)
  }
}

impl<'a> ObjectEntity<'a> {
  fn consume_as_prototype(&self, analyzer: &mut Analyzer<'a>) {
    if self.consumed_as_prototype.replace(true) {
      return;
    }

    self.disable_mangling(analyzer);

    self.prototype.get().consume(analyzer);

    let mut suspended = vec![];
    for property in self.string_keyed.borrow().values() {
      property.consume(analyzer, &mut suspended);
    }
    self.unknown_keyed.borrow().consume(analyzer, &mut suspended);
    analyzer.consume(suspended);
  }

  pub fn is_mangable(&self) -> bool {
    self.mangling_group.is_some_and(|group| group.get().is_some())
  }

  fn check_mangable(
    &self,
    analyzer: &mut Analyzer<'a>,
    literals: &FxHashSet<LiteralEntity>,
  ) -> bool {
    if self.is_mangable() {
      if is_literal_mangable(literals) {
        true
      } else {
        self.disable_mangling(analyzer);
        false
      }
    } else {
      false
    }
  }

  fn disable_mangling(&self, analyzer: &mut Analyzer<'a>) {
    if let Some(group) = self.mangling_group {
      if let Some(group) = group.replace(None) {
        analyzer.mangler.mark_uniqueness_group_non_mangable(group);
      }
    }
  }

  fn add_to_mangling_group(&self, analyzer: &mut Analyzer<'a>, key_atom: MangleAtom) {
    analyzer.mangler.add_to_uniqueness_group(self.mangling_group.unwrap().get().unwrap(), key_atom);
  }
}

impl<'a> Analyzer<'a> {
  pub fn new_empty_object(
    &mut self,
    prototype: ObjectPrototype<'a>,
    mangling_group: Option<ObjectManglingGroupId<'a>>,
  ) -> &'a mut ObjectEntity<'a> {
    self.allocator.alloc(ObjectEntity {
      consumable: true,
      consumed: Cell::new(false),
      consumed_as_prototype: Cell::new(false),
      // deps: Default::default(),
      cf_scope: self.scoping.cf.current_id(),
      object_id: self.scoping.alloc_object_id(),
      string_keyed: RefCell::new(FxHashMap::default()),
      unknown_keyed: RefCell::new(ObjectProperty::default()),
      rest: RefCell::new(None),
      prototype: Cell::new(prototype),
      mangling_group,
    })
  }

  pub fn new_function_object(
    &mut self,
    mangle_node: Option<AstKind2<'a>>,
  ) -> (&'a ObjectEntity<'a>, &'a ObjectEntity<'a>) {
    let mangling_group = if let Some(mangle_node) = mangle_node {
      let (m1, m2) = *self
        .load_data::<Option<(ObjectManglingGroupId, ObjectManglingGroupId)>>(mangle_node)
        .get_or_insert_with(|| {
          (self.new_object_mangling_group(), self.new_object_mangling_group())
        });
      (Some(m1), Some(m2))
    } else {
      (None, None)
    };
    let prototype = self.new_empty_object(
      ObjectPrototype::Builtin(&self.builtins.prototypes.object),
      mangling_group.0,
    );
    let statics = self.new_empty_object(
      ObjectPrototype::Builtin(&self.builtins.prototypes.function),
      mangling_group.1,
    );
    statics.string_keyed.borrow_mut().insert(
      "prototype",
      ObjectProperty {
        definite: true,
        enumerable: false,
        possible_values: vec![ObjectPropertyValue::Field(prototype, false)],
        non_existent: Default::default(),
        key: Some(self.factory.string("prototype")),
        mangling: Some(self.mangler.builtin_atom),
      },
    );
    (statics, prototype)
  }

  pub fn new_object_mangling_group(&mut self) -> ObjectManglingGroupId<'a> {
    self.allocator.alloc(Cell::new(Some(self.mangler.uniqueness_groups.push(Default::default()))))
  }

  pub fn use_mangable_plain_object(
    &mut self,
    dep_id: impl Into<DepId>,
  ) -> &'a mut ObjectEntity<'a> {
    let mangling_group = self
      .load_data::<Option<ObjectManglingGroupId>>(dep_id)
      .get_or_insert_with(|| self.new_object_mangling_group());
    self.new_empty_object(
      ObjectPrototype::Builtin(&self.builtins.prototypes.object),
      Some(*mangling_group),
    )
  }
}
