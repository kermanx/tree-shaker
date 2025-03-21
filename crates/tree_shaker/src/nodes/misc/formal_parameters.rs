use crate::{analyzer::Analyzer, ast::DeclarationKind, entity::Entity, transformer::Transformer};
use oxc::{
  ast::{
    NONE,
    ast::{BindingPatternKind, FormalParameter, FormalParameters},
  },
  span::{GetSpan, SPAN},
};

impl<'a> Analyzer<'a> {
  pub fn exec_formal_parameters(
    &mut self,
    node: &'a FormalParameters<'a>,
    args: Entity<'a>,
    kind: DeclarationKind,
  ) {
    let (elements_init, rest_init, _deps) = args.destruct_as_array(
      self,
      self.factory.empty_consumable,
      node.items.len(),
      node.rest.is_some(),
    );

    for param in &node.items {
      self.declare_binding_pattern(&param.pattern, false, kind);
    }

    for (param, init) in node.items.iter().zip(elements_init) {
      self.init_binding_pattern(&param.pattern, Some(init));
    }

    // In case of `function(x=arguments, y)`, `y` should also be consumed
    if self.call_scope_mut().need_consume_arguments {
      let arguments_consumed = self.consume_arguments();
      assert!(arguments_consumed);
    }

    if let Some(rest) = &node.rest {
      self.declare_binding_rest_element(rest, false, kind);
      self.init_binding_rest_element(rest, rest_init.unwrap());
    }
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_formal_parameters(
    &self,
    node: &'a FormalParameters<'a>,
  ) -> FormalParameters<'a> {
    let FormalParameters { span, items, rest, kind } = node;

    let mut transformed_items = self.ast_builder.vec();

    let mut counting_length = self.config.preserve_function_length;
    let mut used_length = 0;

    for (index, param) in items.iter().enumerate() {
      let FormalParameter { span, decorators, pattern, .. } = param;

      let pattern_was_assignment = matches!(pattern.kind, BindingPatternKind::AssignmentPattern(_));
      let pattern = if let Some(pattern) = self.transform_binding_pattern(pattern, false) {
        used_length = index + 1;
        pattern
      } else {
        self.build_unused_binding_pattern(*span)
      };
      let pattern_is_assignment = matches!(pattern.kind, BindingPatternKind::AssignmentPattern(_));

      transformed_items.push(self.ast_builder.formal_parameter(
        *span,
        self.clone_node(decorators),
        if counting_length && pattern_was_assignment && !pattern_is_assignment {
          self.ast_builder.binding_pattern(
            self.ast_builder.binding_pattern_kind_assignment_pattern(
              pattern.span(),
              pattern,
              self.build_unused_expression(SPAN),
            ),
            NONE,
            false,
          )
        } else {
          pattern
        },
        None,
        false,
        false,
      ));

      if pattern_was_assignment {
        counting_length = false;
      }
      if counting_length {
        used_length = index + 1;
      }
    }

    let transformed_rest = match rest {
      Some(rest) => self.transform_binding_rest_element(rest, false),
      None => None,
    };

    transformed_items.truncate(used_length);

    self.ast_builder.formal_parameters(*span, *kind, transformed_items, transformed_rest)
  }
}
