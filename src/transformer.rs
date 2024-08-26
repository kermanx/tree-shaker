use crate::{
  analyzer::Analyzer,
  ast::AstType2,
  data::{DataPlaceholder, ExtraData, ReferredNodes},
  entity::dep::EntityDepNode,
};
use oxc::{
  allocator::Allocator,
  ast::{
    ast::{BindingPattern, Expression, Program, TSTypeAnnotation, UnaryOperator},
    AstBuilder,
  },
  span::{GetSpan, SourceType, Span, SPAN},
};
use std::{
  hash::{DefaultHasher, Hasher}, mem, sync::{atomic::{AtomicUsize, Ordering}, LazyLock}
};

pub(crate) struct Transformer<'a> {
  allocator: &'a Allocator,
  pub(crate) ast_builder: AstBuilder<'a>,
  pub(crate) data: ExtraData<'a>,
  pub(crate) referred_nodes: ReferredNodes<'a>,
}

impl<'a> Transformer<'a> {
  pub fn new(analyzer: Analyzer<'a>) -> Self {
    let Analyzer { allocator, data, referred_nodes, .. } = analyzer;
    Transformer { allocator, ast_builder: AstBuilder::new(allocator), data, referred_nodes }
  }

  pub fn transform_program(&mut self, ast: &'a mut Program<'a>) -> Program<'a> {
    let Program { span, source_type, hashbang, directives, body, .. } = mem::replace(
      ast,
      self.ast_builder.program(
        SPAN,
        SourceType::default(),
        None,
        self.ast_builder.vec(),
        self.ast_builder.vec(),
      ),
    );
    let body = self.transform_statements(body);
    self.ast_builder.program(span, source_type, hashbang, directives, body)
  }
}

impl<'a> Transformer<'a> {
  pub(crate) fn build_unused_binding_pattern(&self, span: Span) -> BindingPattern<'a> {
    let mut hasher = DefaultHasher::new();
    hasher.write_u32(span.start);
    hasher.write_u32(span.end);
    let name = format!("__unused_{:04X}", hasher.finish() % 0xFFFF);
    self.ast_builder.binding_pattern(
      self.ast_builder.binding_pattern_kind_binding_identifier(span, name),
      None::<TSTypeAnnotation>,
      false,
    )
  }

  pub(crate) fn build_negate_expression(&self, expression: Expression<'a>) -> Expression<'a> {
    self.ast_builder.expression_unary(expression.span(), UnaryOperator::LogicalNot, expression)
  }
}

impl<'a> Transformer<'a> {
  pub(crate) fn get_data_by_span<D: Default + 'a>(&self, ast_type: AstType2, span: Span) -> &'a D {
    let existing = self.data.get(&ast_type).and_then(|map| map.get(&span));
    match existing {
      Some(boxed) => unsafe { mem::transmute::<&DataPlaceholder<'_>, &D>(boxed.as_ref()) },
      None => self.allocator.alloc(D::default()),
    }
  }

  pub(crate) fn get_data<D: Default + 'a>(&self, ast_type: AstType2, node: &dyn GetSpan) -> &'a D {
    self.get_data_by_span(ast_type, node.span())
  }
}

impl<'a> Transformer<'a> {
  pub(crate) fn is_referred(&self, node: EntityDepNode<'a>) -> bool {
    self.referred_nodes.contains(&node)
  }
}
