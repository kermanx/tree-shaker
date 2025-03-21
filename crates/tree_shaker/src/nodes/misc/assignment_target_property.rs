use crate::{analyzer::Analyzer, ast::AstKind2, entity::Entity, transformer::Transformer};
use oxc::{
  ast::ast::{
    AssignmentTargetProperty, AssignmentTargetPropertyIdentifier, AssignmentTargetPropertyProperty,
  },
  span::{GetSpan, SPAN},
};

#[derive(Debug, Default)]
struct IdentifierData {
  need_init: bool,
}

impl<'a> Analyzer<'a> {
  /// Returns the key
  pub fn exec_assignment_target_property(
    &mut self,
    node: &'a AssignmentTargetProperty<'a>,
    value: Entity<'a>,
  ) -> Entity<'a> {
    let dep = AstKind2::AssignmentTargetProperty(node);
    match node {
      AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(node) => {
        let key = self.factory.string(node.binding.name.as_str());

        let value = value.get_property(self, dep, key);

        let (need_init, value) = if let Some(init) = &node.init {
          self.exec_with_default(init, value)
        } else {
          (false, value)
        };

        let data =
          self.load_data::<IdentifierData>(AstKind2::AssignmentTargetPropertyIdentifier(node));
        data.need_init |= need_init;

        self.exec_identifier_reference_write(&node.binding, value);

        key
      }
      AssignmentTargetProperty::AssignmentTargetPropertyProperty(node) => {
        self.push_dependent_cf_scope(value);
        let key = self.exec_property_key(&node.name);
        self.pop_cf_scope();

        let value = value.get_property(self, dep, key);
        self.exec_assignment_target_maybe_default(&node.binding, value);
        key
      }
    }
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_assignment_target_property(
    &self,
    node: &'a AssignmentTargetProperty<'a>,
  ) -> Option<AssignmentTargetProperty<'a>> {
    let need_binding = self.is_referred(AstKind2::AssignmentTargetProperty(node));
    match node {
      AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(node) => {
        let data =
          self.get_data::<IdentifierData>(AstKind2::AssignmentTargetPropertyIdentifier(node));

        let AssignmentTargetPropertyIdentifier { span, binding, init } = node.as_ref();

        let binding_span = binding.span();
        let binding_name = binding.name.as_str();
        let binding = self.transform_identifier_reference_write(binding);
        let init = data
          .need_init
          .then(|| {
            init.as_ref().and_then(|init| self.transform_expression(init, binding.is_some()))
          })
          .flatten();

        if need_binding && binding.is_none() {
          Some(self.ast_builder.assignment_target_property_assignment_target_property_property(
            *span,
            self.ast_builder.property_key_static_identifier(binding_span, binding_name),
            if let Some(init) = init {
              self.ast_builder.assignment_target_maybe_default_assignment_target_with_default(
                *span,
                self.build_unused_assignment_target(SPAN),
                init,
              )
            } else {
              self.build_unused_assignment_target(SPAN).into()
            },
            false,
          ))
        } else if binding.is_some() || init.is_some() {
          Some(self.ast_builder.assignment_target_property_assignment_target_property_identifier(
            *span,
            binding.map_or_else(
              || self.build_unused_identifier_reference_write(binding_span),
              |binding| binding.unbox(),
            ),
            init,
          ))
        } else {
          None
        }
      }
      AssignmentTargetProperty::AssignmentTargetPropertyProperty(node) => {
        let AssignmentTargetPropertyProperty { span, name, binding, computed } = node.as_ref();

        let name_span = name.span();
        let binding = self.transform_assignment_target_maybe_default(binding, need_binding);
        if let Some(binding) = binding {
          let name = self.transform_property_key(name, true).unwrap();
          Some(self.ast_builder.assignment_target_property_assignment_target_property_property(
            *span, name, binding, *computed,
          ))
        } else {
          self.transform_property_key(name, false).map(|name| {
            self.ast_builder.assignment_target_property_assignment_target_property_property(
              *span,
              name,
              self.build_unused_assignment_target(name_span).into(),
              *computed,
            )
          })
        }
      }
    }
  }
}
