use super::{ObjectEntity, ObjectProperty, ObjectPropertyValue, ObjectPrototype};
use crate::{
  analyzer::Analyzer,
  consumable::{Consumable, ConsumableCollector, ConsumableTrait},
  entity::{Entity, LiteralEntity, consumed_object},
  mangling::{MangleAtom, MangleConstraint},
  scope::CfScopeKind,
  utils::Found,
};

pub struct PendingSetter<'a> {
  pub indeterminate: bool,
  pub dep: Consumable<'a>,
  pub setter: Entity<'a>,
}

impl<'a> ObjectEntity<'a> {
  pub fn set_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    key: Entity<'a>,
    value: Entity<'a>,
  ) {
    if self.consumed.get() {
      return consumed_object::set_property(analyzer, dep, key, value);
    }

    let (has_exhaustive, mut indeterminate, exec_deps) =
      analyzer.pre_mutate_object(self.cf_scope, self.object_id);

    if has_exhaustive {
      self.consume(analyzer);
      return consumed_object::set_property(analyzer, dep, key, value);
    }

    let mut setters = vec![];

    if self.lookup_unknown_keyed_setters(analyzer, &mut setters).may_found() {
      indeterminate = true;
    }

    let value = analyzer.factory.computed(value, (exec_deps, dep));
    let non_mangable_value = analyzer.factory.computed(value, key);

    if let Some(key_literals) = key.get_to_literals(analyzer) {
      let mut string_keyed = self.string_keyed.borrow_mut();
      let mut rest = self.rest.borrow_mut();

      indeterminate |= key_literals.len() > 1;

      let mangable = self.check_mangable(analyzer, &key_literals);
      let value = if mangable { value } else { non_mangable_value };

      for key_literal in key_literals {
        match key_literal {
          LiteralEntity::String(key_str, key_atom) => {
            if let Some(property) = string_keyed.get_mut(key_str) {
              let value = if mangable {
                let prev_key = property.key.unwrap();
                let prev_atom = property.mangling.unwrap();
                analyzer.factory.mangable(
                  value,
                  (prev_key, key),
                  MangleConstraint::Eq(prev_atom, key_atom.unwrap()),
                )
              } else {
                value
              };
              property.set(analyzer, indeterminate, value, &mut setters);
              if property.definite {
                continue;
              }
            }

            if let Some(rest) = &mut *rest {
              rest.set(analyzer, true, value, &mut setters);
              continue;
            }

            let found =
              self.lookup_string_keyed_setters_on_proto(analyzer, key_str, key_atom, &mut setters);
            if found.must_found() {
              continue;
            }

            if mangable {
              self.add_to_mangling_group(analyzer, key_atom.unwrap());
            }
            string_keyed.insert(
              key_str,
              ObjectProperty {
                definite: !indeterminate && found.must_not_found(),
                enumerable: true, /* TODO: Object.defineProperty */
                possible_values: analyzer.factory.vec1(ObjectPropertyValue::Field(value, false)),
                non_existent: ConsumableCollector::new(analyzer.factory.vec()),
                key: Some(key),
                mangling: mangable.then(|| key_atom.unwrap()),
              },
            );
          }
          LiteralEntity::Symbol(_, _) => todo!(),
          _ => unreachable!("Invalid property key"),
        }
      }
    } else {
      self.disable_mangling(analyzer);

      indeterminate = true;

      let mut unknown_keyed = self.unknown_keyed.borrow_mut();
      unknown_keyed.possible_values.push(ObjectPropertyValue::Field(non_mangable_value, false));

      let mut string_keyed = self.string_keyed.borrow_mut();
      for property in string_keyed.values_mut() {
        property.set(analyzer, true, non_mangable_value, &mut setters);
      }

      if let Some(rest) = &mut *self.rest.borrow_mut() {
        rest.set(analyzer, true, non_mangable_value, &mut setters);
      }

      self.lookup_any_string_keyed_setters_on_proto(analyzer, &mut setters);
    }

    if !setters.is_empty() {
      let indeterminate = indeterminate || setters.len() > 1 || setters[0].indeterminate;
      analyzer.push_cf_scope_with_deps(
        CfScopeKind::Dependent,
        analyzer.factory.vec1(analyzer.consumable((dep, key))),
        if indeterminate { None } else { Some(false) },
      );
      for s in setters {
        s.setter.call_as_setter(analyzer, s.dep, self.into(), non_mangable_value);
      }
      analyzer.pop_cf_scope();
    }
  }

  fn lookup_unknown_keyed_setters(
    &self,
    analyzer: &mut Analyzer<'a>,
    setters: &mut Vec<PendingSetter<'a>>,
  ) -> Found {
    let mut found = Found::False;

    found += self.unknown_keyed.borrow_mut().lookup_setters(analyzer, setters);

    match self.prototype.get() {
      ObjectPrototype::ImplicitOrNull => {}
      ObjectPrototype::Builtin(_) => {}
      ObjectPrototype::Custom(prototype) => {
        found += prototype.lookup_unknown_keyed_setters(analyzer, setters);
      }
      ObjectPrototype::Unknown(dep) => {
        setters.push(PendingSetter {
          indeterminate: true,
          dep,
          setter: analyzer.factory.computed_unknown(dep),
        });
        found = Found::Unknown;
      }
    }

    found
  }

  fn lookup_string_keyed_setters_on_proto(
    &self,
    analyzer: &mut Analyzer<'a>,
    key_str: &str,
    mut key_atom: Option<MangleAtom>,
    setters: &mut Vec<PendingSetter<'a>>,
  ) -> Found {
    match self.prototype.get() {
      ObjectPrototype::ImplicitOrNull => Found::False,
      ObjectPrototype::Builtin(_) => Found::False, // FIXME: Setters on builtin prototypes
      ObjectPrototype::Custom(prototype) => {
        let found1 = if let Some(property) = prototype.string_keyed.borrow_mut().get_mut(key_str) {
          if prototype.is_mangable() {
            if key_atom.is_none() {
              prototype.disable_mangling(analyzer);
            }
          } else {
            key_atom = None;
          }
          let found = property.lookup_setters(analyzer, setters);
          if property.definite && found.must_found() {
            return Found::True;
          }
          if found == Found::False { Found::False } else { Found::Unknown }
        } else {
          Found::False
        };

        let found2 =
          prototype.lookup_string_keyed_setters_on_proto(analyzer, key_str, key_atom, setters);

        found1 + found2
      }
      ObjectPrototype::Unknown(_dep) => Found::Unknown,
    }
  }

  fn lookup_any_string_keyed_setters_on_proto(
    &self,
    analyzer: &mut Analyzer<'a>,
    setters: &mut Vec<PendingSetter<'a>>,
  ) {
    match self.prototype.get() {
      ObjectPrototype::ImplicitOrNull => {}
      ObjectPrototype::Builtin(_) => {}
      ObjectPrototype::Custom(prototype) => {
        if prototype.is_mangable() {
          prototype.disable_mangling(analyzer);
        }

        for property in prototype.string_keyed.borrow_mut().values_mut() {
          property.lookup_setters(analyzer, setters);
        }

        prototype.lookup_any_string_keyed_setters_on_proto(analyzer, setters);
      }
      ObjectPrototype::Unknown(_dep) => {}
    }
  }
}
