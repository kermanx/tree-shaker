use super::ObjectEntity;
use crate::{
  analyzer::Analyzer,
  consumable::Consumable,
  entity::{consumed_object, object::ObjectPrototype, Entity, LiteralEntity},
  mangling::MangleAtom,
  scope::CfScopeKind,
};

pub(crate) struct GetPropertyContext<'a> {
  pub key: Entity<'a>,
  pub values: Vec<Entity<'a>>,
  pub getters: Vec<Entity<'a>>,
  pub extra_deps: Vec<Consumable<'a>>,
}

impl<'a> ObjectEntity<'a> {
  pub fn get_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    key: Entity<'a>,
  ) -> Entity<'a> {
    if self.consumed.get() {
      return consumed_object::get_property(self, analyzer, dep, key);
    }

    analyzer.mark_object_property_exhaustive_read(self.cf_scope, self.object_id);

    let mut mangable = false;
    let mut context =
      GetPropertyContext { key, values: vec![], getters: vec![], extra_deps: vec![] };

    let mut check_rest = false;
    if let Some(key_literals) = key.get_to_literals(analyzer) {
      mangable = self.check_mangable(analyzer, &key_literals);
      for key_literal in key_literals {
        match key_literal {
          LiteralEntity::String(key_str, key_atom) => {
            if !self.get_string_keyed(analyzer, &mut context, key_str, key_atom) {
              check_rest = true;
            }
          }
          LiteralEntity::Symbol(_, _) => todo!(),
          _ => unreachable!("Invalid property key"),
        }
      }
    } else {
      self.disable_mangling(analyzer);

      self.get_any_string_keyed(analyzer, &mut context);

      // TODO: prototype? Use a config IMO
      // Either:
      // - Skip prototype
      // - Return unknown and call all getters

      check_rest = true;
    }

    if check_rest {
      let mut rest = self.rest.borrow_mut();
      if let Some(rest) = &mut *rest {
        rest.get(analyzer, &mut context, None);
      } else {
        context.values.push(analyzer.factory.undefined);
      }
    }

    self.get_unknown_keyed(analyzer, &mut context);

    if !context.getters.is_empty() {
      let indeterminate = check_rest || !context.values.is_empty() || context.getters.len() > 1;
      analyzer.push_cf_scope_with_deps(
        CfScopeKind::Dependent,
        vec![if mangable { dep } else { analyzer.consumable((dep, key)) }],
        if indeterminate { None } else { Some(false) },
      );
      for getter in context.getters {
        context.values.push(getter.call_as_getter(
          analyzer,
          analyzer.factory.empty_consumable,
          self,
        ));
      }
      analyzer.pop_cf_scope();
    }

    let value = analyzer.factory.try_union(context.values).unwrap_or(analyzer.factory.undefined);
    if mangable {
      analyzer.factory.computed(value, analyzer.consumable((context.extra_deps, dep)))
    } else {
      analyzer.factory.computed(value, analyzer.consumable((context.extra_deps, dep, key)))
    }
  }

  fn get_string_keyed(
    &self,
    analyzer: &mut Analyzer<'a>,
    context: &mut GetPropertyContext<'a>,
    key_str: &str,
    mut key_atom: Option<MangleAtom>,
  ) -> bool {
    if self.is_mangable() {
      if key_atom.is_none() {
        self.disable_mangling(analyzer);
      }
    } else {
      key_atom = None;
    }

    let mut string_keyed = self.string_keyed.borrow_mut();
    if let Some(property) = string_keyed.get_mut(key_str) {
      property.get(analyzer, context, key_atom);
      if property.definite {
        return true;
      }
    }

    match self.prototype.get() {
      ObjectPrototype::ImplicitOrNull => false,
      ObjectPrototype::Builtin(prototype) => {
        if let Some(value) = prototype.get_string_keyed(key_str) {
          context.values.push(if let Some(key_atom) = key_atom {
            analyzer.factory.computed(value, key_atom)
          } else {
            value
          });
          true
        } else {
          false
        }
      }
      ObjectPrototype::Custom(prototype) => {
        prototype.get_string_keyed(analyzer, context, key_str, key_atom)
      }
      ObjectPrototype::Unknown(_unknown) => false,
    }
  }

  fn get_any_string_keyed(&self, analyzer: &Analyzer<'a>, context: &mut GetPropertyContext<'a>) {
    for property in self.string_keyed.borrow_mut().values_mut() {
      property.get(analyzer, context, None);
    }
    match self.prototype.get() {
      ObjectPrototype::ImplicitOrNull => {}
      ObjectPrototype::Builtin(_prototype) => {
        // TODO: Control via an option
      }
      ObjectPrototype::Custom(prototype) => prototype.get_any_string_keyed(analyzer, context),
      ObjectPrototype::Unknown(_dep) => {}
    }
  }

  fn get_unknown_keyed(&self, analyzer: &Analyzer<'a>, context: &mut GetPropertyContext<'a>) {
    let mut unknown_keyed = self.unknown_keyed.borrow_mut();
    unknown_keyed.get(analyzer, context, None);
    match self.prototype.get() {
      ObjectPrototype::ImplicitOrNull => {}
      ObjectPrototype::Builtin(_) => {}
      ObjectPrototype::Custom(prototype) => prototype.get_unknown_keyed(analyzer, context),
      ObjectPrototype::Unknown(dep) => context.values.push(analyzer.factory.computed_unknown(dep)),
    }
  }
}
