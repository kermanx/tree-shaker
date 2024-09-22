use crate::{
  analyzer::Analyzer,
  ast::{AstType2, DeclarationKind},
  entity::entity::Entity,
  transformer::Transformer,
};
use oxc::{
  ast::{
    ast::{BindingPatternKind, FormalParameter, FormalParameters},
    NONE,
  },
  span::{GetSpan, SPAN},
};

const AST_TYPE: AstType2 = AstType2::FormalParameter;

#[derive(Debug, Default)]
pub struct Data<'a> {
  elements_init: Vec<Vec<Entity<'a>>>,
  rest_init: Vec<Entity<'a>>,
}

impl<'a> Analyzer<'a> {
  pub fn exec_formal_parameters(
    &mut self,
    node: &'a FormalParameters<'a>,
    args: Entity<'a>,
    kind: DeclarationKind,
  ) {
    let (elements_init, rest_init) = args.destruct_as_array(self, (), node.items.len());

    let data = self.load_data::<Data>(AST_TYPE, node);
    data.elements_init.push(elements_init.clone());
    data.rest_init.push(rest_init.clone());

    for (param, _) in node.items.iter().zip(&elements_init) {
      self.declare_binding_pattern(&param.pattern, false, kind);
    }

    for (param, init) in node.items.iter().zip(elements_init) {
      self.init_binding_pattern(&param.pattern, Some(init));
    }

    if let Some(rest) = &node.rest {
      self.declare_binding_rest_element(rest, false, kind);
      self.init_binding_rest_element(rest, rest_init);
    }
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_formal_parameters(
    &self,
    node: &'a FormalParameters<'a>,
  ) -> FormalParameters<'a> {
    let data = self.get_data::<Data>(AST_TYPE, node);

    let FormalParameters { span, items, rest, kind, .. } = node;

    let mut transformed_items = self.ast_builder.vec();

    let mut counting_length = self.config.preserve_function_length;
    let mut used_length = 0;

    for (index, param) in items.iter().enumerate() {
      let FormalParameter { span, decorators, pattern, .. } = param;

      let pattern_was_assignment = matches!(pattern.kind, BindingPatternKind::AssignmentPattern(_));
      let pattern = if let Some(pattern) = self.transform_binding_pattern(pattern, false) {
        used_length = index + 1;
        for dep in &data.elements_init {
          dep[index].refer_dep_shallow(self);
        }
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
