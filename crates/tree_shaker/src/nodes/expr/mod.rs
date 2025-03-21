mod array_expression;
mod arrow_function_expression;
mod assignment_expression;
mod await_expression;
mod binary_expression;
mod call_expression;
mod chain_expression;
mod conditional_expression;
mod import_expression;
mod literals;
mod logical_expression;
mod member_expression;
mod meta_property;
mod new_expression;
mod object_expression;
mod parenthesized_expression;
mod private_in_expression;
mod sequence_expression;
mod super_expression;
mod tagged_template_expression;
mod template_literal;
mod this_expression;
mod unary_expression;
mod update_expression;
mod yield_expression;

use crate::{
  analyzer::Analyzer, build_effect, entity::Entity, transformer::Transformer, utils::ast::AstKind2,
};
use oxc::{
  ast::{ast::Expression, match_member_expression},
  span::GetSpan,
};
use oxc_syntax::operator::{AssignmentOperator, UnaryOperator};

impl<'a> Analyzer<'a> {
  pub fn exec_expression(&mut self, node: &'a Expression<'a>) -> Entity<'a> {
    self.push_span(node);
    let value = match node {
      match_member_expression!(Expression) => {
        self.exec_member_expression_read(node.to_member_expression(), false).0
      }
      Expression::StringLiteral(node) => self.exec_string_literal(node),
      Expression::NumericLiteral(node) => self.exec_numeric_literal(node),
      Expression::BigIntLiteral(node) => self.exc_big_int_literal(node),
      Expression::BooleanLiteral(node) => self.exec_boolean_literal(node),
      Expression::NullLiteral(node) => self.exec_null_literal(node),
      Expression::RegExpLiteral(node) => self.exec_regexp_literal(node),
      Expression::TemplateLiteral(node) => self.exec_template_literal(node),
      Expression::Identifier(node) => self.exec_identifier_reference_read(node),
      Expression::FunctionExpression(node) => self.exec_function(node),
      Expression::ArrowFunctionExpression(node) => self.exec_arrow_function_expression(node),
      Expression::UnaryExpression(node) => self.exec_unary_expression(node),
      Expression::UpdateExpression(node) => self.exec_update_expression(node),
      Expression::BinaryExpression(node) => self.exec_binary_expression(node),
      Expression::LogicalExpression(node) => self.exec_logical_expression(node),
      Expression::ConditionalExpression(node) => self.exec_conditional_expression(node),
      Expression::CallExpression(node) => self.exec_call_expression(node),
      Expression::TaggedTemplateExpression(node) => self.exec_tagged_template_expression(node),
      Expression::AwaitExpression(node) => self.exec_await_expression(node),
      Expression::YieldExpression(node) => self.exec_yield_expression(node),
      Expression::ObjectExpression(node) => self.exec_object_expression(node),
      Expression::ArrayExpression(node) => self.exec_array_expression(node),
      Expression::ParenthesizedExpression(node) => self.exec_parenthesized_expression(node),
      Expression::SequenceExpression(node) => self.exec_sequence_expression(node),
      Expression::AssignmentExpression(node) => self.exec_assignment_expression(node),
      Expression::ChainExpression(node) => self.exec_chain_expression(node),
      Expression::ImportExpression(node) => self.exec_import_expression(node),
      Expression::MetaProperty(node) => self.exec_meta_property(node),
      Expression::NewExpression(node) => self.exec_new_expression(node),
      Expression::ClassExpression(node) => self.exec_class(node),
      Expression::ThisExpression(node) => self.exec_this_expression(node),
      Expression::Super(node) => self.exec_super(node),
      Expression::PrivateInExpression(node) => self.exec_private_in_expression(node),

      Expression::JSXElement(node) => self.exec_jsx_element(node),
      Expression::JSXFragment(node) => self.exec_jsx_fragment(node),

      Expression::V8IntrinsicExpression(_)
      | Expression::TSAsExpression(_)
      | Expression::TSInstantiationExpression(_)
      | Expression::TSTypeAssertion(_)
      | Expression::TSNonNullExpression(_)
      | Expression::TSSatisfiesExpression(_) => unreachable!(),
    };
    self.pop_span();
    self.try_fold_node(AstKind2::Expression(node), value)
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_expression(
    &self,
    node: &'a Expression<'a>,
    need_val: bool,
  ) -> Option<Expression<'a>> {
    let span = node.span();
    let literal = need_val.then(|| self.build_folded_expr(AstKind2::Expression(node))).flatten();
    let need_val = need_val && literal.is_none();

    let inner = match node {
      match_member_expression!(Expression) => {
        self.transform_member_expression_read(node.to_member_expression(), need_val)
      }
      Expression::StringLiteral(_)
      | Expression::NumericLiteral(_)
      | Expression::BigIntLiteral(_)
      | Expression::BooleanLiteral(_)
      | Expression::NullLiteral(_)
      | Expression::RegExpLiteral(_) => need_val.then(|| self.clone_node(node)),
      Expression::TemplateLiteral(node) => self.transform_template_literal(node, need_val),
      Expression::Identifier(node) => {
        self.transform_identifier_reference_read(node, need_val).map(Expression::Identifier)
      }
      Expression::FunctionExpression(node) => {
        self.transform_function(node, need_val).map(Expression::FunctionExpression)
      }
      Expression::ArrowFunctionExpression(node) => {
        self.transform_arrow_function_expression(node, need_val)
      }
      Expression::UnaryExpression(node) => self.transform_unary_expression(node, need_val),
      Expression::UpdateExpression(node) => self.transform_update_expression(node, need_val),
      Expression::BinaryExpression(node) => self.transform_binary_expression(node, need_val),
      Expression::LogicalExpression(node) => self.transform_logical_expression(node, need_val),
      Expression::ConditionalExpression(node) => {
        self.transform_conditional_expression(node, need_val)
      }
      Expression::CallExpression(node) => self.transform_call_expression(node, need_val),
      Expression::TaggedTemplateExpression(node) => {
        self.transform_tagged_template_expression(node, need_val)
      }
      Expression::AwaitExpression(node) => self.transform_await_expression(node, need_val),
      Expression::YieldExpression(node) => self.transform_yield_expression(node, need_val),
      Expression::ObjectExpression(node) => self.transform_object_expression(node, need_val),
      Expression::ArrayExpression(node) => self.transform_array_expression(node, need_val),
      Expression::ParenthesizedExpression(node) => {
        self.transform_parenthesized_expression(node, need_val)
      }
      Expression::SequenceExpression(node) => self.transform_sequence_expression(node, need_val),
      Expression::AssignmentExpression(node) => {
        self.transform_assignment_expression(node, need_val)
      }
      Expression::ChainExpression(node) => self.transform_chain_expression(node, need_val),
      Expression::ImportExpression(node) => self.transform_import_expression(node, need_val),
      Expression::MetaProperty(node) => self.transform_meta_property(node, need_val),
      Expression::NewExpression(node) => self.transform_new_expression(node, need_val),
      Expression::ClassExpression(node) => {
        self.transform_class(node, need_val).map(Expression::ClassExpression)
      }
      Expression::ThisExpression(node) => self.transform_this_expression(node, need_val),
      Expression::Super(node) => self.transform_super(node, need_val),
      Expression::PrivateInExpression(node) => self.transform_private_in_expression(node, need_val),

      Expression::JSXElement(node) => self.transform_jsx_element(node, need_val),
      Expression::JSXFragment(node) => self.transform_jsx_fragment(node, need_val),

      Expression::V8IntrinsicExpression(_)
      | Expression::TSAsExpression(_)
      | Expression::TSInstantiationExpression(_)
      | Expression::TSTypeAssertion(_)
      | Expression::TSNonNullExpression(_)
      | Expression::TSSatisfiesExpression(_) => unreachable!(),
    };

    if let Some(literal) = literal {
      if let Some(inner) = inner {
        // Avoid `(a = 1, 1)`
        if let Expression::AssignmentExpression(node) = &inner {
          if node.operator == AssignmentOperator::Assign && node.right.is_literal() {
            return Some(inner);
          }
        }

        // Avoid `(void effect(), void 0)`
        if let Expression::UnaryExpression(node) = &inner {
          if node.operator == UnaryOperator::Void {
            if let Expression::UnaryExpression(literal) = &literal {
              if literal.operator == UnaryOperator::Void {
                return Some(inner);
              }
            }
          }
        }

        Some(build_effect!(&self.ast_builder, span, inner; literal))
      } else {
        Some(literal)
      }
    } else {
      inner
    }
  }
}
