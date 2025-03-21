#![allow(clippy::unnecessary_unwrap)]

use super::{ObjectEntity, ObjectProperty, ObjectPropertyValue};
use crate::{
  analyzer::Analyzer,
  consumable::{ConsumableCollector, ConsumeTrait},
  entity::{Entity, EntityFactory, LiteralEntity},
  mangling::MangleConstraint,
};
use oxc::ast::ast::PropertyKind;

impl<'a> ObjectEntity<'a> {
  pub fn init_property(
    &self,
    analyzer: &mut Analyzer<'a>,
    kind: PropertyKind,
    key: Entity<'a>,
    value: Entity<'a>,
    definite: bool,
  ) {
    if let Some(key_literals) = key.get_to_literals(analyzer) {
      let mangable = self.check_mangable(analyzer, &key_literals);
      let value = if mangable { value } else { analyzer.factory.computed(value, key) };

      let definite = definite && key_literals.len() == 1;
      for key_literal in key_literals {
        match key_literal {
          LiteralEntity::String(key_str, key_atom) => {
            let mut string_keyed = self.string_keyed.borrow_mut();
            let existing = string_keyed.get_mut(key_str);
            let reused_property = definite
              .then(|| {
                existing.as_ref().and_then(|existing| {
                  for property in existing.possible_values.iter() {
                    if let ObjectPropertyValue::Property(getter, setter) = property {
                      return Some((*getter, *setter));
                    }
                  }
                  None
                })
              })
              .flatten();
            let constraint = if mangable {
              if let Some(existing) = &existing {
                let prev_atom = existing.mangling.unwrap();
                Some(MangleConstraint::Eq(prev_atom, key_atom.unwrap()))
              } else {
                self.add_to_mangling_group(analyzer, key_atom.unwrap());
                None
              }
            } else {
              None
            };
            let value = if let Some(constraint) = constraint {
              analyzer.factory.computed(value, constraint)
            } else {
              value
            };
            let property_val = match kind {
              PropertyKind::Init => ObjectPropertyValue::Field(value, false),
              PropertyKind::Get => ObjectPropertyValue::Property(
                Some(value),
                reused_property.and_then(|(_, setter)| setter),
              ),
              PropertyKind::Set => ObjectPropertyValue::Property(
                reused_property.and_then(|(getter, _)| getter),
                Some(value),
              ),
            };
            let existing = string_keyed.get_mut(key_str);
            if definite || existing.is_none() {
              let property = ObjectProperty {
                definite,
                enumerable: true,
                possible_values: analyzer.factory.vec1(property_val),
                non_existent: ConsumableCollector::new(analyzer.factory.vec()),
                key: Some(key),
                mangling: mangable.then(|| key_atom.unwrap()),
              };
              string_keyed.insert(key_str, property);
            } else {
              existing.unwrap().possible_values.push(property_val);
            }
          }
          LiteralEntity::Symbol(_key, _) => todo!(),
          _ => unreachable!("Invalid property key"),
        }
      }
    } else {
      self.disable_mangling(analyzer);
      let value = analyzer.factory.computed(value, key);
      let property_val = match kind {
        PropertyKind::Init => ObjectPropertyValue::Field(value, false),
        PropertyKind::Get => ObjectPropertyValue::Property(Some(value), None),
        PropertyKind::Set => ObjectPropertyValue::Property(None, Some(value)),
      };
      self.unknown_keyed.borrow_mut().possible_values.push(property_val);
    }
  }

  pub fn init_spread(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: impl ConsumeTrait<'a> + 'a,
    argument: Entity<'a>,
  ) {
    let (properties, deps) = argument.enumerate_properties(analyzer, dep);
    for (definite, key, value) in properties {
      self.init_property(analyzer, PropertyKind::Init, key, value, definite);
    }
    self.unknown_keyed.borrow_mut().non_existent.push(deps);
  }

  pub fn init_rest(&self, factory: &EntityFactory<'a>, property: ObjectPropertyValue<'a>) {
    assert_eq!(self.mangling_group, None);
    let mut rest = self.rest.borrow_mut();
    if let Some(rest) = &mut *rest {
      rest.possible_values.push(property);
    } else {
      *rest = Some(ObjectProperty {
        definite: false,
        enumerable: true,
        possible_values: factory.vec1(property),
        non_existent: ConsumableCollector::new(factory.vec()),
        key: None,
        mangling: None,
      });
    }
  }
}
