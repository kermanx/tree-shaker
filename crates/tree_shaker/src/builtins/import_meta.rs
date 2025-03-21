use super::{Builtins, constants::IMPORT_META_OBJECT_ID, prototypes::BuiltinPrototypes};
use crate::{
  consumable::ConsumableCollector,
  entity::{Entity, EntityFactory, ObjectProperty, ObjectPropertyValue, ObjectPrototype},
};

impl<'a> Builtins<'a> {
  pub fn create_import_meta(
    factory: &'a EntityFactory<'a>,
    _prototypes: &'a BuiltinPrototypes<'a>,
  ) -> Entity<'a> {
    let object =
      factory.builtin_object(IMPORT_META_OBJECT_ID, ObjectPrototype::ImplicitOrNull, true);
    object.init_rest(
      factory,
      ObjectPropertyValue::Property(
        Some(factory.immutable_unknown),
        Some(factory.immutable_unknown),
      ),
    );

    // import.meta.url
    object.string_keyed.borrow_mut().insert(
      "url",
      ObjectProperty {
        definite: true,
        enumerable: true,
        possible_values: factory.vec1(ObjectPropertyValue::Property(
          Some(factory.implemented_builtin_fn("import.meta.url", |analyzer, _, _, _| {
            analyzer.factory.unknown_string
          })),
          None,
        )),
        non_existent: ConsumableCollector::new(factory.vec()),
        key: None,
        mangling: None,
      },
    );

    object.into()
  }
}
