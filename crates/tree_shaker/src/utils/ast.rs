use crate::{analyzer::Analyzer, consumable::ConsumableTrait};
use oxc::{
  allocator::Vec,
  ast::ast::*,
  span::{GetSpan, SPAN},
};
use std::fmt::{self, Debug};

pub type Arguments<'a> = Vec<'a, Argument<'a>>;

#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum AstKind2<'a> {
  // Special
  Environment,
  Index(usize),

  BooleanLiteral(&'a BooleanLiteral),
  NullLiteral(&'a NullLiteral),
  NumericLiteral(&'a NumericLiteral<'a>),
  BigIntLiteral(&'a BigIntLiteral<'a>),
  RegExpLiteral(&'a RegExpLiteral<'a>),
  StringLiteral(&'a StringLiteral<'a>),
  Program(&'a Program<'a>),
  IdentifierName(&'a IdentifierName<'a>),
  IdentifierReference(&'a IdentifierReference<'a>),
  BindingIdentifier(&'a BindingIdentifier<'a>),
  LabelIdentifier(&'a LabelIdentifier<'a>),
  ThisExpression(&'a ThisExpression),
  ArrayExpression(&'a ArrayExpression<'a>),
  ArrayExpressionElement(&'a ArrayExpressionElement<'a>),
  Elision(&'a Elision),
  ObjectExpression(&'a ObjectExpression<'a>),
  ObjectProperty(&'a ObjectProperty<'a>),
  PropertyKey(&'a PropertyKey<'a>),
  TemplateLiteral(&'a TemplateLiteral<'a>),
  TaggedTemplateExpression(&'a TaggedTemplateExpression<'a>),
  MemberExpression(&'a MemberExpression<'a>),
  CallExpression(&'a CallExpression<'a>),
  NewExpression(&'a NewExpression<'a>),
  MetaProperty(&'a MetaProperty<'a>),
  SpreadElement(&'a SpreadElement<'a>),
  Argument(&'a Argument<'a>),
  UpdateExpression(&'a UpdateExpression<'a>),
  UnaryExpression(&'a UnaryExpression<'a>),
  BinaryExpression(&'a BinaryExpression<'a>),
  PrivateInExpression(&'a PrivateInExpression<'a>),
  LogicalExpression(&'a LogicalExpression<'a>),
  ConditionalExpression(&'a ConditionalExpression<'a>),
  AssignmentExpression(&'a AssignmentExpression<'a>),
  AssignmentTarget(&'a AssignmentTarget<'a>),
  SimpleAssignmentTarget(&'a SimpleAssignmentTarget<'a>),
  AssignmentTargetPattern(&'a AssignmentTargetPattern<'a>),
  ArrayAssignmentTarget(&'a ArrayAssignmentTarget<'a>),
  ObjectAssignmentTarget(&'a ObjectAssignmentTarget<'a>),
  AssignmentTargetWithDefault(&'a AssignmentTargetWithDefault<'a>),
  SequenceExpression(&'a SequenceExpression<'a>),
  Super(&'a Super),
  AwaitExpression(&'a AwaitExpression<'a>),
  ChainExpression(&'a ChainExpression<'a>),
  ParenthesizedExpression(&'a ParenthesizedExpression<'a>),
  Directive(&'a Directive<'a>),
  Hashbang(&'a Hashbang<'a>),
  BlockStatement(&'a BlockStatement<'a>),
  VariableDeclaration(&'a VariableDeclaration<'a>),
  VariableDeclarator(&'a VariableDeclarator<'a>),
  EmptyStatement(&'a EmptyStatement),
  ExpressionStatement(&'a ExpressionStatement<'a>),
  IfStatement(&'a IfStatement<'a>),
  DoWhileStatement(&'a DoWhileStatement<'a>),
  WhileStatement(&'a WhileStatement<'a>),
  ForStatement(&'a ForStatement<'a>),
  ForStatementInit(&'a ForStatementInit<'a>),
  ForInStatement(&'a ForInStatement<'a>),
  ForOfStatement(&'a ForOfStatement<'a>),
  ContinueStatement(&'a ContinueStatement<'a>),
  BreakStatement(&'a BreakStatement<'a>),
  ReturnStatement(&'a ReturnStatement<'a>),
  WithStatement(&'a WithStatement<'a>),
  SwitchStatement(&'a SwitchStatement<'a>),
  SwitchCase(&'a SwitchCase<'a>),
  LabeledStatement(&'a LabeledStatement<'a>),
  ThrowStatement(&'a ThrowStatement<'a>),
  TryStatement(&'a TryStatement<'a>),
  FinallyClause(&'a BlockStatement<'a>),
  CatchClause(&'a CatchClause<'a>),
  CatchParameter(&'a CatchParameter<'a>),
  DebuggerStatement(&'a DebuggerStatement),
  AssignmentPattern(&'a AssignmentPattern<'a>),
  ObjectPattern(&'a ObjectPattern<'a>),
  ArrayPattern(&'a ArrayPattern<'a>),
  BindingRestElement(&'a BindingRestElement<'a>),
  Function(&'a Function<'a>),
  FormalParameters(&'a FormalParameters<'a>),
  FormalParameter(&'a FormalParameter<'a>),
  FunctionBody(&'a FunctionBody<'a>),
  ArrowFunctionExpression(&'a ArrowFunctionExpression<'a>),
  YieldExpression(&'a YieldExpression<'a>),
  Class(&'a Class<'a>),
  ClassHeritage(&'a Expression<'a>),
  ClassBody(&'a ClassBody<'a>),
  MethodDefinition(&'a MethodDefinition<'a>),
  PropertyDefinition(&'a PropertyDefinition<'a>),
  PrivateIdentifier(&'a PrivateIdentifier<'a>),
  StaticBlock(&'a StaticBlock<'a>),
  ModuleDeclaration(&'a ModuleDeclaration<'a>),
  ImportExpression(&'a ImportExpression<'a>),
  ImportDeclaration(&'a ImportDeclaration<'a>),
  ImportSpecifier(&'a ImportSpecifier<'a>),
  ImportDefaultSpecifier(&'a ImportDefaultSpecifier<'a>),
  ImportNamespaceSpecifier(&'a ImportNamespaceSpecifier<'a>),
  ExportNamedDeclaration(&'a ExportNamedDeclaration<'a>),
  ExportDefaultDeclaration(&'a ExportDefaultDeclaration<'a>),
  ExportAllDeclaration(&'a ExportAllDeclaration<'a>),
  ExportSpecifier(&'a ExportSpecifier<'a>),
  JSXAttributeItem(&'a JSXAttributeItem<'a>),
  JSXMemberExpression(&'a JSXMemberExpression<'a>),
  JsxExpressionContainer(&'a JSXExpressionContainer<'a>),

  // extras
  Expression(&'a Expression<'a>),
  SwitchCaseTest(&'a SwitchCase<'a>),
  AssignmentTargetProperty(&'a AssignmentTargetProperty<'a>),
  AssignmentTargetPropertyIdentifier(&'a AssignmentTargetPropertyIdentifier<'a>),
  AssignmentTargetRest(&'a AssignmentTargetRest<'a>),
  BindingProperty(&'a BindingProperty<'a>),
  Callee(&'a Expression<'a>),
  ClassConstructor(&'a Class<'a>),
  ExpressionInTaggedTemplate(&'a Expression<'a>),
  LogicalExpressionLeft(&'a LogicalExpression<'a>),
  LogicalAssignmentExpressionLeft(&'a AssignmentExpression<'a>),
  JSXOpeningElement(&'a JSXOpeningElement<'a>),
  JSXAttributeName(&'a JSXAttributeName<'a>),
}

impl GetSpan for AstKind2<'_> {
  fn span(&self) -> Span {
    match self {
      AstKind2::Environment | AstKind2::Index(_) => SPAN,
      AstKind2::BooleanLiteral(node) => node.span(),
      AstKind2::NullLiteral(node) => node.span(),
      AstKind2::NumericLiteral(node) => node.span(),
      AstKind2::BigIntLiteral(node) => node.span(),
      AstKind2::RegExpLiteral(node) => node.span(),
      AstKind2::StringLiteral(node) => node.span(),
      AstKind2::Program(node) => node.span(),
      AstKind2::IdentifierName(node) => node.span(),
      AstKind2::IdentifierReference(node) => node.span(),
      AstKind2::BindingIdentifier(node) => node.span(),
      AstKind2::LabelIdentifier(node) => node.span(),
      AstKind2::ThisExpression(node) => node.span(),
      AstKind2::ArrayExpression(node) => node.span(),
      AstKind2::ArrayExpressionElement(node) => node.span(),
      AstKind2::Elision(node) => node.span(),
      AstKind2::ObjectExpression(node) => node.span(),
      AstKind2::ObjectProperty(node) => node.span(),
      AstKind2::PropertyKey(node) => node.span(),
      AstKind2::TemplateLiteral(node) => node.span(),
      AstKind2::TaggedTemplateExpression(node) => node.span(),
      AstKind2::MemberExpression(node) => node.span(),
      AstKind2::CallExpression(node) => node.span(),
      AstKind2::NewExpression(node) => node.span(),
      AstKind2::MetaProperty(node) => node.span(),
      AstKind2::SpreadElement(node) => node.span(),
      AstKind2::Argument(node) => node.span(),
      AstKind2::UpdateExpression(node) => node.span(),
      AstKind2::UnaryExpression(node) => node.span(),
      AstKind2::BinaryExpression(node) => node.span(),
      AstKind2::PrivateInExpression(node) => node.span(),
      AstKind2::LogicalExpression(node) => node.span(),
      AstKind2::ConditionalExpression(node) => node.span(),
      AstKind2::AssignmentExpression(node) => node.span(),
      AstKind2::AssignmentTarget(node) => node.span(),
      AstKind2::SimpleAssignmentTarget(node) => node.span(),
      AstKind2::AssignmentTargetPattern(node) => node.span(),
      AstKind2::ArrayAssignmentTarget(node) => node.span(),
      AstKind2::ObjectAssignmentTarget(node) => node.span(),
      AstKind2::AssignmentTargetWithDefault(node) => node.span(),
      AstKind2::SequenceExpression(node) => node.span(),
      AstKind2::Super(node) => node.span(),
      AstKind2::AwaitExpression(node) => node.span(),
      AstKind2::ChainExpression(node) => node.span(),
      AstKind2::ParenthesizedExpression(node) => node.span(),
      AstKind2::Directive(node) => node.span(),
      AstKind2::Hashbang(node) => node.span(),
      AstKind2::BlockStatement(node) => node.span(),
      AstKind2::VariableDeclaration(node) => node.span(),
      AstKind2::VariableDeclarator(node) => node.span(),
      AstKind2::EmptyStatement(node) => node.span(),
      AstKind2::ExpressionStatement(node) => node.span(),
      AstKind2::IfStatement(node) => node.span(),
      AstKind2::DoWhileStatement(node) => node.span(),
      AstKind2::WhileStatement(node) => node.span(),
      AstKind2::ForStatement(node) => node.span(),
      AstKind2::ForStatementInit(node) => node.span(),
      AstKind2::ForInStatement(node) => node.span(),
      AstKind2::ForOfStatement(node) => node.span(),
      AstKind2::ContinueStatement(node) => node.span(),
      AstKind2::BreakStatement(node) => node.span(),
      AstKind2::ReturnStatement(node) => node.span(),
      AstKind2::WithStatement(node) => node.span(),
      AstKind2::SwitchStatement(node) => node.span(),
      AstKind2::SwitchCase(node) => node.span(),
      AstKind2::LabeledStatement(node) => node.span(),
      AstKind2::ThrowStatement(node) => node.span(),
      AstKind2::TryStatement(node) => node.span(),
      AstKind2::FinallyClause(node) => node.span(),
      AstKind2::CatchClause(node) => node.span(),
      AstKind2::CatchParameter(node) => node.span(),
      AstKind2::DebuggerStatement(node) => node.span(),
      AstKind2::AssignmentPattern(node) => node.span(),
      AstKind2::ObjectPattern(node) => node.span(),
      AstKind2::ArrayPattern(node) => node.span(),
      AstKind2::BindingRestElement(node) => node.span(),
      AstKind2::Function(node) => node.span(),
      AstKind2::FormalParameters(node) => node.span(),
      AstKind2::FormalParameter(node) => node.span(),
      AstKind2::FunctionBody(node) => node.span(),
      AstKind2::ArrowFunctionExpression(node) => node.span(),
      AstKind2::YieldExpression(node) => node.span(),
      AstKind2::Class(node) => node.span(),
      AstKind2::ClassHeritage(node) => node.span(),
      AstKind2::ClassBody(node) => node.span(),
      AstKind2::MethodDefinition(node) => node.span(),
      AstKind2::PropertyDefinition(node) => node.span(),
      AstKind2::PrivateIdentifier(node) => node.span(),
      AstKind2::StaticBlock(node) => node.span(),
      AstKind2::ModuleDeclaration(node) => node.span(),
      AstKind2::ImportExpression(node) => node.span(),
      AstKind2::ImportDeclaration(node) => node.span(),
      AstKind2::ImportSpecifier(node) => node.span(),
      AstKind2::ImportDefaultSpecifier(node) => node.span(),
      AstKind2::ImportNamespaceSpecifier(node) => node.span(),
      AstKind2::ExportNamedDeclaration(node) => node.span(),
      AstKind2::ExportDefaultDeclaration(node) => node.span(),
      AstKind2::ExportAllDeclaration(node) => node.span(),
      AstKind2::ExportSpecifier(node) => node.span(),
      AstKind2::JSXAttributeItem(node) => node.span(),
      AstKind2::JSXMemberExpression(node) => node.span(),
      AstKind2::JsxExpressionContainer(node) => node.span(),
      AstKind2::Expression(node) => node.span(),
      AstKind2::SwitchCaseTest(node) => node.span(),
      AstKind2::AssignmentTargetProperty(node) => node.span(),
      AstKind2::AssignmentTargetPropertyIdentifier(node) => node.span(),
      AstKind2::AssignmentTargetRest(node) => node.span(),
      AstKind2::BindingProperty(node) => node.span(),
      AstKind2::Callee(node) => node.span(),
      AstKind2::ClassConstructor(node) => node.span(),
      AstKind2::ExpressionInTaggedTemplate(node) => node.span(),
      AstKind2::LogicalExpressionLeft(node) => node.span(),
      AstKind2::LogicalAssignmentExpressionLeft(node) => node.span(),
      AstKind2::JSXOpeningElement(node) => node.span(),
      AstKind2::JSXAttributeName(node) => node.span(),
    }
  }
}

impl fmt::Debug for AstKind2<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.span().fmt(f)
  }
}

impl<'a> ConsumableTrait<'a> for AstKind2<'a> {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    analyzer.refer_dep(*self);
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeclarationKind {
  Var,
  UntrackedVar,
  Let,
  Const,
  Function,
  NamedFunctionInBody,
  Class,
  Import,
  Caught,
  FunctionParameter,
  ArrowFunctionParameter,
}

impl DeclarationKind {
  pub fn is_var(self) -> bool {
    matches!(self, DeclarationKind::Var | DeclarationKind::UntrackedVar)
  }

  pub fn is_untracked(self) -> bool {
    matches!(self, DeclarationKind::UntrackedVar)
  }

  pub fn is_const(self) -> bool {
    matches!(self, DeclarationKind::Const | DeclarationKind::NamedFunctionInBody)
  }

  pub fn is_redeclarable(self) -> bool {
    matches!(
      self,
      DeclarationKind::Var
        | DeclarationKind::UntrackedVar
        | DeclarationKind::Function
        | DeclarationKind::Class
    )
  }

  pub fn is_shadowable(self) -> bool {
    self.is_redeclarable()
      || matches!(
        self,
        DeclarationKind::FunctionParameter
          | DeclarationKind::ArrowFunctionParameter
          | DeclarationKind::Caught
      )
  }
}
