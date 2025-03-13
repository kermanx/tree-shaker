use crate::transformer::Transformer;
use oxc::ast::{
  ast::{
    BindingPatternKind, ClassElement, Function, MethodDefinition, MethodDefinitionKind,
    PropertyDefinitionType,
  },
  NONE,
};

impl<'a> Transformer<'a> {
  pub fn transform_method_definition(
    &self,
    node: &'a MethodDefinition<'a>,
  ) -> Option<ClassElement<'a>> {
    let MethodDefinition {
      r#type,
      span,
      decorators,
      key,
      value,
      kind,
      computed,
      r#static,
      r#override,
      optional,
      accessibility,
    } = node;

    if let Some(mut transformed_value) = self.transform_function(value, false) {
      let key = if node.kind.is_constructor() {
        self.clone_node(key)
      } else {
        self.transform_property_key(key, true).unwrap()
      };

      if *kind == MethodDefinitionKind::Set {
        self.patch_method_definition_params(value, &mut transformed_value);
      }

      Some(self.ast_builder.class_element_method_definition(
        *span,
        *r#type,
        self.clone_node(decorators),
        key,
        transformed_value,
        *kind,
        *computed,
        *r#static,
        *r#override,
        *optional,
        *accessibility,
      ))
    } else {
      let key = self.transform_property_key(key, false);
      return key.map(|key| {
        self.ast_builder.class_element_property_definition(
          *span,
          PropertyDefinitionType::PropertyDefinition,
          self.ast_builder.vec(),
          key,
          None,
          true,
          *r#static,
          false,
          false,
          false,
          false,
          false,
          NONE,
          None,
        )
      });
    }
  }

  /// It is possible that `set a(param) {}` has been optimized to `set a() {}`.
  /// This function patches the parameter list if it is empty.
  pub fn patch_method_definition_params(
    &self,
    original_node: &'a Function<'a>,
    transformed_node: &mut Function<'a>,
  ) {
    if !transformed_node.params.has_parameter() {
      let span = original_node.span;
      let original_param = &original_node.params.items[0];
      transformed_node.params.items.push(self.ast_builder.formal_parameter(
        span,
        self.ast_builder.vec(),
        if self.config.preserve_function_length
          && matches!(original_param.pattern.kind, BindingPatternKind::AssignmentPattern(_))
        {
          self.build_unused_assignment_binding_pattern(span)
        } else {
          self.build_unused_binding_pattern(span)
        },
        None,
        false,
        false,
      ));
    }
  }
}
