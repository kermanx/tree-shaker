use crate::{
  builtins::{Builtins, constants::OBJECT_CONSTRUCTOR_OBJECT_ID},
  entity::{Entity, LiteralEntity, ObjectPropertyValue, ObjectPrototype, TypeofResult},
  init_namespace,
};
use std::borrow::BorrowMut;

impl<'a> Builtins<'a> {
  pub fn init_object_constructor(&mut self) {
    let factory = self.factory;

    let object = factory.builtin_object(
      OBJECT_CONSTRUCTOR_OBJECT_ID,
      ObjectPrototype::Builtin(&self.prototypes.function),
      false,
    );
    object.init_rest(factory, ObjectPropertyValue::Field(factory.immutable_unknown, true));

    init_namespace!(object, factory, {
      "prototype" => factory.immutable_unknown,
      "assign" => self.create_object_assign_impl(),
      "keys" => self.create_object_keys_impl(),
      "values" => self.create_object_values_impl(),
      "entries" => self.create_object_entries_impl(),
      "freeze" => self.create_object_freeze_impl(),
      "defineProperty" => self.create_object_define_property_impl(),
    });

    self.globals.borrow_mut().insert("Object", object.into());
  }

  fn create_object_assign_impl(&self) -> Entity<'a> {
    self.factory.implemented_builtin_fn("Object.assign", |analyzer, dep, _, args| {
      let (known, rest, deps) = args.iterate(analyzer, dep);

      if known.len() < 2 {
        return analyzer.factory.computed_unknown((dep, args));
      }

      let target = known[0];

      let mut assign = |source: Entity<'a>, indeterminate: bool| {
        let (properties, deps) = source.enumerate_properties(analyzer, dep);
        for (definite, key, value) in properties {
          if indeterminate || !definite {
            analyzer.push_indeterminate_cf_scope();
          }
          target.set_property(analyzer, deps, key, value);
          if indeterminate || !definite {
            analyzer.pop_cf_scope();
          }
        }
      };

      for source in &known[1..] {
        assign(*source, false);
      }
      if let Some(rest) = rest {
        assign(rest, true);
      }

      analyzer.factory.computed(target, deps)
    })
  }

  fn create_object_keys_impl(&self) -> Entity<'a> {
    self.factory.implemented_builtin_fn("Object.keys", |analyzer, dep, _, args| {
      let object = args.destruct_as_array(analyzer, dep, 1, false).0[0];
      let (properties, deps) = object.enumerate_properties(analyzer, dep);

      let array = analyzer.new_empty_array();

      for (_, key, value) in properties {
        if key.test_typeof().contains(TypeofResult::String) {
          array.init_rest(analyzer.factory.computed(key.get_to_string(analyzer), value));
        }
      }

      analyzer.factory.computed(array.into(), deps)
    })
  }

  fn create_object_values_impl(&self) -> Entity<'a> {
    self.factory.implemented_builtin_fn("Object.values", |analyzer, dep, _, args| {
      let object = args.destruct_as_array(analyzer, dep, 1, false).0[0];
      let (properties, deps) = object.enumerate_properties(analyzer, dep);

      let array = analyzer.new_empty_array();

      for (_, _, value) in properties {
        array.init_rest(value);
      }

      analyzer.factory.computed(array.into(), deps)
    })
  }

  fn create_object_entries_impl(&self) -> Entity<'a> {
    self.factory.implemented_builtin_fn("Object.entries", |analyzer, dep, _, args| {
      let object = args.destruct_as_array(analyzer, dep, 1, false).0[0];
      let (properties, deps) = object.enumerate_properties(analyzer, dep);

      let array = analyzer.new_empty_array();

      for (_, key, value) in properties {
        let entry = analyzer.new_empty_array();
        entry.push_element(key.get_to_string(analyzer));
        entry.push_element(value);
        array.init_rest(entry.into());
      }

      analyzer.factory.computed(array.into(), deps)
    })
  }

  fn create_object_freeze_impl(&self) -> Entity<'a> {
    self.factory.implemented_builtin_fn("Object.freeze", |analyzer, dep, _, args| {
      let object = args.destruct_as_array(analyzer, dep, 1, false).0[0];
      if analyzer.config.preserve_writablity {
        object.unknown_mutate(analyzer, dep);
        object
      } else {
        // TODO: Actually freeze the object
        analyzer.factory.computed(object, dep)
      }
    })
  }

  fn create_object_define_property_impl(&self) -> Entity<'a> {
    self.factory.implemented_builtin_fn("Object.defineProperty", |analyzer, dep, _, args| {
      let [object, key, descriptor] = args.destruct_as_array(analyzer, dep, 3, false).0[..] else {
        unreachable!()
      };
      let key = key.get_to_property_key(analyzer);

      'trackable: {
        if analyzer.config.preserve_writablity {
          break 'trackable;
        }
        if key.get_literal(analyzer).is_none() {
          break 'trackable;
        }
        let (properties, dep) = descriptor.enumerate_properties(analyzer, dep);
        let mut value = analyzer.factory.undefined;
        for (definite, key, value2) in properties {
          if !definite {
            break 'trackable;
          }
          let Some(LiteralEntity::String(key_str, _)) = key.get_literal(analyzer) else {
            break 'trackable;
          };
          match key_str {
            "value" => {
              value = self.factory.computed(value2, (key, value));
            }
            "get" => {
              // FIXME: This is not safe, but OK for now.
              value = self.factory.computed_unknown((value2, key, value));
            }
            "set" | "enumerable" | "configurable" | "writable" => {
              // TODO: actually handle these
              value = self.factory.computed(value, (key, value2));
            }
            _ => {}
          }
        }
        object.set_property(
          analyzer,
          analyzer.factory.consumable((dep, descriptor.shallow_dep())),
          key,
          value,
        );
        return object;
      }

      object.unknown_mutate(analyzer, (dep, key, descriptor));
      object
    })
  }
}
