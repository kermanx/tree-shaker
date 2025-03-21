#[macro_export]
macro_rules! init_namespace {
  ($ns:expr, $factory:expr, { $($k:expr => $v:expr,)* }) => {
    {
      use $crate::entity::{ObjectProperty, ObjectPropertyValue};
      use $crate::consumable::ConsumableCollector;
      let mut string_keyed = $ns.string_keyed.borrow_mut();
      $(string_keyed.insert(
        $k,
        ObjectProperty {
          definite: true,
          enumerable: false,
          possible_values:  $factory.vec1(ObjectPropertyValue::Field($v, true)),
          non_existent: ConsumableCollector::new($factory.vec()),
          key: None,
          mangling: None,
        },
      );)*
    }
  };
}

#[macro_export]
macro_rules! init_object {
  ($ns:expr, $factory:expr, { $($k:expr => $v:expr,)* }) => {
    {
      use $crate::entity::{ObjectProperty, ObjectPropertyValue};
      use $crate::consumable::ConsumableCollector;
      let mut string_keyed = $ns.string_keyed.borrow_mut();
      $(string_keyed.insert(
        $k,
        ObjectProperty {
          definite: true,
          enumerable: true,
          possible_values: $factory.vec1(ObjectPropertyValue::Field($v, false)),
          non_existent: ConsumableCollector::new($factory.vec()),
          key: None,
          mangling: None,
        },
      );)*
    }
  };
}

#[macro_export]
macro_rules! init_map {
  ($map:expr, { $($k:expr => $v:expr,)* }) => {
    {
      $($map.insert($k, $v);)*
    }
  };
}
