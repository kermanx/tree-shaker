use super::ObjectValue;
use crate::{
  analyzer::Analyzer,
  dep::{CustomDepTrait, Dep},
  entity::Entity,
  mangling::{MangleConstraint, ManglingDep},
  value::{LiteralValue, consumed_object},
};

impl<'a> ObjectValue<'a> {
  pub fn delete_property(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>, key: Entity<'a>) {
    if self.consumed.get() {
      return consumed_object::delete_property(analyzer, dep, key);
    }

    let (has_exhaustive, indeterminate, exec_deps) =
      analyzer.pre_mutate_object(self.cf_scope, self.object_id);

    if has_exhaustive {
      self.consume(analyzer);
      return consumed_object::delete_property(analyzer, dep, key);
    }

    let dep = analyzer.dep((exec_deps, dep));

    {
      let mut unknown_keyed = self.unknown_keyed.borrow_mut();
      if !unknown_keyed.possible_values.is_empty() {
        unknown_keyed.delete(true, analyzer.dep((dep, key)));
      }
    }

    if let Some(key_literals) = key.get_to_literals(analyzer) {
      let indeterminate = indeterminate || key_literals.len() > 1;
      let mangable = self.check_mangable(analyzer, &key_literals);
      let dep = if mangable { dep } else { analyzer.dep((dep, key)) };

      let mut string_keyed = self.string_keyed.borrow_mut();
      let mut rest = self.rest.borrow_mut();
      for key_literal in key_literals {
        match key_literal {
          LiteralValue::String(key_str, key_atom) => {
            if let Some(property) = string_keyed.get_mut(key_str) {
              property.delete(
                indeterminate,
                if mangable {
                  let prev_key = property.key.unwrap();
                  let prev_atom = property.mangling.unwrap();
                  analyzer.dep((
                    dep,
                    ManglingDep {
                      deps: (prev_key, key),
                      constraint: MangleConstraint::Eq(prev_atom, key_atom.unwrap()),
                    },
                  ))
                } else {
                  dep
                },
              );
            } else if let Some(rest) = &mut *rest {
              rest.delete(true, analyzer.dep((dep, key)));
            } else if mangable {
              self.add_to_mangling_group(analyzer, key_atom.unwrap());
            }
          }
          LiteralValue::Symbol(_, _) => todo!(),
          _ => unreachable!("Invalid property key"),
        }
      }
    } else {
      self.disable_mangling(analyzer);

      let dep = analyzer.dep((dep, key));

      let mut string_keyed = self.string_keyed.borrow_mut();
      for property in string_keyed.values_mut() {
        property.delete(true, dep);
      }
    }
  }
}
