use super::{
  consumed_object, never::NeverEntity, Entity, EntityTrait, EnumeratedProperties, IteratedElements,
  TypeofResult,
};
use crate::{
  analyzer::Analyzer,
  builtins::BuiltinPrototype,
  consumable::Consumable,
  mangling::{MangleAtom, MangleConstraint},
  transformer::Transformer,
  utils::F64WithEq,
};
use oxc::{
  allocator::Allocator,
  ast::ast::{BigintBase, Expression, NumberBase, UnaryOperator},
  semantic::SymbolId,
  span::{Atom, Span, SPAN},
};
use oxc_ecmascript::StringToNumber;
use oxc_syntax::number::ToJsString;
use rustc_hash::FxHashSet;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum LiteralEntity<'a> {
  String(&'a str, Option<MangleAtom>),
  Number(F64WithEq, Option<&'a str>),
  BigInt(&'a str),
  Boolean(bool),
  Symbol(SymbolId, &'a str),
  Infinity(bool),
  NaN,
  Null,
  Undefined,
}

impl<'a> EntityTrait<'a> for LiteralEntity<'a> {
  fn consume(&'a self, analyzer: &mut Analyzer<'a>) {
    if let LiteralEntity::String(_, Some(atom)) = self {
      analyzer.consume(*atom);
    }
  }

  fn consume_mangable(&'a self, _analyzer: &mut Analyzer<'a>) -> bool {
    // No effect
    !matches!(self, LiteralEntity::String(_, Some(_)))
  }

  fn unknown_mutate(&'a self, _analyzer: &mut Analyzer<'a>, _dep: Consumable<'a>) {
    // No effect
  }

  fn get_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    key: Entity<'a>,
  ) -> Entity<'a> {
    if matches!(self, LiteralEntity::Null | LiteralEntity::Undefined) {
      analyzer.throw_builtin_error("Cannot get property of null or undefined");
      if analyzer.config.preserve_exceptions {
        consumed_object::get_property(self, analyzer, dep, key)
      } else {
        analyzer.factory.never
      }
    } else {
      let prototype = self.get_prototype(analyzer);
      let dep = analyzer.consumable((self, dep, key));
      if let Some(key_literals) = key.get_to_literals(analyzer) {
        let mut values = vec![];
        for key_literal in key_literals {
          if let Some(property) = self.get_known_instance_property(analyzer, key_literal) {
            values.push(property);
          } else if let Some(property) = prototype.get_literal_keyed(key_literal) {
            values.push(property);
          } else {
            values.push(analyzer.factory.unmatched_prototype_property);
          }
        }
        analyzer.factory.computed_union(values, dep)
      } else {
        analyzer.factory.computed_unknown(dep)
      }
    }
  }

  fn set_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    key: Entity<'a>,
    value: Entity<'a>,
  ) {
    if matches!(self, LiteralEntity::Null | LiteralEntity::Undefined) {
      analyzer.throw_builtin_error("Cannot set property of null or undefined");
      if analyzer.config.preserve_exceptions {
        consumed_object::set_property(analyzer, dep, key, value)
      }
    } else {
      // No effect
    }
  }

  fn enumerate_properties(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
  ) -> EnumeratedProperties<'a> {
    if let LiteralEntity::String(value, atom) = self {
      let dep = analyzer.consumable((dep, *atom));
      if value.len() <= analyzer.config.max_simple_string_length {
        (
          value
            .char_indices()
            .map(|(i, c)| {
              (
                true,
                analyzer.factory.string(analyzer.allocator.alloc(i.to_string())),
                analyzer.factory.string(analyzer.allocator.alloc(c.to_string())),
              )
            })
            .collect(),
          dep,
        )
      } else {
        analyzer.factory.computed_unknown_string(self).enumerate_properties(analyzer, dep)
      }
    } else {
      // No effect
      (vec![], dep)
    }
  }

  fn delete_property(&'a self, analyzer: &mut Analyzer<'a>, dep: Consumable<'a>, _key: Entity<'a>) {
    if matches!(self, LiteralEntity::Null | LiteralEntity::Undefined) {
      analyzer.throw_builtin_error("Cannot delete property of null or undefined");
      if analyzer.config.preserve_exceptions {
        analyzer.consume(dep);
      }
    } else {
      // No effect
    }
  }

  fn call(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    this: Entity<'a>,
    args: Entity<'a>,
  ) -> Entity<'a> {
    analyzer.throw_builtin_error(format!("Cannot call a non-function object {:?}", self));
    if analyzer.config.preserve_exceptions {
      consumed_object::call(self, analyzer, dep, this, args)
    } else {
      analyzer.factory.never
    }
  }

  fn construct(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    args: Entity<'a>,
  ) -> Entity<'a> {
    analyzer.throw_builtin_error(format!("Cannot construct a non-constructor object {:?}", self));
    if analyzer.config.preserve_exceptions {
      consumed_object::construct(self, analyzer, dep, args)
    } else {
      analyzer.factory.never
    }
  }

  fn jsx(&'a self, analyzer: &mut Analyzer<'a>, attributes: Entity<'a>) -> Entity<'a> {
    analyzer.factory.computed_unknown((self, attributes))
  }

  fn r#await(&'a self, analyzer: &mut Analyzer<'a>, dep: Consumable<'a>) -> Entity<'a> {
    analyzer.factory.computed(self, dep)
  }

  fn iterate(&'a self, analyzer: &mut Analyzer<'a>, dep: Consumable<'a>) -> IteratedElements<'a> {
    match self {
      LiteralEntity::String(value, atom) => (
        vec![],
        (!value.is_empty()).then_some(analyzer.factory.unknown_string),
        analyzer.consumable((self, dep, *atom)),
      ),
      _ => {
        analyzer.throw_builtin_error("Cannot iterate over a non-iterable object");
        if analyzer.config.preserve_exceptions {
          self.consume(analyzer);
          consumed_object::iterate(analyzer, dep)
        } else {
          NeverEntity.iterate(analyzer, dep)
        }
      }
    }
  }

  fn get_destructable(&'a self, _analyzer: &Analyzer<'a>, dep: Consumable<'a>) -> Consumable<'a> {
    dep
  }

  fn get_typeof(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.string(self.test_typeof().to_string().unwrap())
  }

  fn get_to_string(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.alloc(LiteralEntity::String(
      self.to_string(analyzer.allocator),
      if let LiteralEntity::String(_, Some(atom)) = self { Some(*atom) } else { None },
    ))
  }

  fn get_to_numeric(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    match self {
      LiteralEntity::Number(_, _)
      | LiteralEntity::BigInt(_)
      | LiteralEntity::NaN
      | LiteralEntity::Infinity(_) => self,
      LiteralEntity::Boolean(value) => {
        if *value {
          analyzer.factory.number(1.0, Some("1"))
        } else {
          analyzer.factory.number(0.0, Some("0"))
        }
      }
      LiteralEntity::String(str, atom) => {
        let val = str.string_to_number();
        analyzer.factory.computed(
          if val.is_nan() { analyzer.factory.nan } else { analyzer.factory.number(val, None) },
          *atom,
        )
      }
      LiteralEntity::Null => analyzer.factory.number(0.0, Some("0")),
      LiteralEntity::Symbol(_, _) => {
        // TODO: warn: TypeError: Cannot convert a Symbol value to a number
        analyzer.factory.unknown()
      }
      LiteralEntity::Undefined => analyzer.factory.nan,
    }
  }

  fn get_to_boolean(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    match self.test_truthy() {
      Some(value) => analyzer.factory.boolean(value),
      None => analyzer.factory.computed_unknown_boolean(self),
    }
  }

  fn get_to_property_key(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    match self {
      LiteralEntity::Symbol(_, _) => self,
      _ => self.get_to_string(analyzer),
    }
  }

  fn get_to_jsx_child(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    if (TypeofResult::String | TypeofResult::Number).contains(self.test_typeof()) {
      self.get_to_string(analyzer)
    } else {
      analyzer.factory.string("")
    }
  }

  fn get_to_literals(&'a self, _analyzer: &Analyzer<'a>) -> Option<FxHashSet<LiteralEntity<'a>>> {
    let mut result = FxHashSet::default();
    result.insert(*self);
    Some(result)
  }

  fn get_literal(&'a self, _analyzer: &Analyzer<'a>) -> Option<LiteralEntity<'a>> {
    Some(*self)
  }

  fn get_own_keys(&'a self, _analyzer: &Analyzer<'a>) -> Option<Vec<(bool, Entity<'a>)>> {
    match self {
      LiteralEntity::String(_, _) => None,
      _ => Some(vec![]),
    }
  }

  fn test_typeof(&self) -> TypeofResult {
    match self {
      LiteralEntity::String(_, _) => TypeofResult::String,
      LiteralEntity::Number(_, _) => TypeofResult::Number,
      LiteralEntity::BigInt(_) => TypeofResult::BigInt,
      LiteralEntity::Boolean(_) => TypeofResult::Boolean,
      LiteralEntity::Symbol(_, _) => TypeofResult::Symbol,
      LiteralEntity::Infinity(_) => TypeofResult::Number,
      LiteralEntity::NaN => TypeofResult::Number,
      LiteralEntity::Null => TypeofResult::Object,
      LiteralEntity::Undefined => TypeofResult::Undefined,
    }
  }

  fn test_truthy(&self) -> Option<bool> {
    Some(match self {
      LiteralEntity::String(value, _) => !value.is_empty(),
      LiteralEntity::Number(value, _) => *value != 0.0.into() && *value != (-0.0).into(),
      LiteralEntity::BigInt(value) => !value.chars().all(|c| c == '0'),
      LiteralEntity::Boolean(value) => *value,
      LiteralEntity::Symbol(_, _) => true,
      LiteralEntity::Infinity(_) => true,
      LiteralEntity::NaN | LiteralEntity::Null | LiteralEntity::Undefined => false,
    })
  }

  fn test_nullish(&self) -> Option<bool> {
    Some(matches!(self, LiteralEntity::Null | LiteralEntity::Undefined))
  }
}

impl<'a> LiteralEntity<'a> {
  pub fn build_expr(
    &self,
    transformer: &Transformer<'a>,
    span: Span,
    atom: Option<MangleAtom>,
  ) -> Expression<'a> {
    let ast_builder = transformer.ast_builder;
    match self {
      LiteralEntity::String(value, _) => {
        let mut mangler = transformer.mangler.borrow_mut();
        let mangled = atom.and_then(|a| mangler.resolve(a)).unwrap_or(value);
        ast_builder.expression_string_literal(span, mangled, None)
      }
      LiteralEntity::Number(value, raw) => {
        let negated = value.0.is_sign_negative();
        let absolute = ast_builder.expression_numeric_literal(
          span,
          value.0.abs(),
          raw.map(Atom::from),
          NumberBase::Decimal,
        );
        if negated {
          ast_builder.expression_unary(span, UnaryOperator::UnaryNegation, absolute)
        } else {
          absolute
        }
      }
      LiteralEntity::BigInt(value) => {
        ast_builder.expression_big_int_literal(span, *value, BigintBase::Decimal)
      }
      LiteralEntity::Boolean(value) => ast_builder.expression_boolean_literal(span, *value),
      LiteralEntity::Symbol(_, _) => unreachable!("Cannot build expression for Symbol"),
      LiteralEntity::Infinity(positive) => {
        if *positive {
          ast_builder.expression_identifier_reference(span, "Infinity")
        } else {
          ast_builder.expression_unary(
            span,
            UnaryOperator::UnaryNegation,
            ast_builder.expression_identifier_reference(span, "Infinity"),
          )
        }
      }
      LiteralEntity::NaN => ast_builder.expression_identifier_reference(span, "NaN"),
      LiteralEntity::Null => ast_builder.expression_null_literal(span),
      LiteralEntity::Undefined => ast_builder.expression_unary(
        span,
        UnaryOperator::Void,
        ast_builder.expression_numeric_literal(SPAN, 0.0, Some("0".into()), NumberBase::Decimal),
      ),
    }
  }

  pub fn can_build_expr(&self, analyzer: &Analyzer<'a>) -> bool {
    let config = &analyzer.config;
    match self {
      LiteralEntity::String(value, _) => value.len() <= config.max_simple_string_length,
      LiteralEntity::Number(value, _) => {
        value.0.fract() == 0.0
          && config.min_simple_number_value <= (value.0 as i64)
          && (value.0 as i64) <= config.max_simple_number_value
      }
      LiteralEntity::BigInt(_) => false,
      LiteralEntity::Boolean(_) => true,
      LiteralEntity::Symbol(_, _) => false,
      LiteralEntity::Infinity(_) => true,
      LiteralEntity::NaN => true,
      LiteralEntity::Null => true,
      LiteralEntity::Undefined => true,
    }
  }

  pub fn to_string(self, allocator: &'a Allocator) -> &'a str {
    match self {
      LiteralEntity::String(value, _) => value,
      LiteralEntity::Number(value, str_rep) => {
        str_rep.unwrap_or_else(|| allocator.alloc(value.0.to_js_string()))
      }
      LiteralEntity::BigInt(value) => value,
      LiteralEntity::Boolean(value) => {
        if value {
          "true"
        } else {
          "false"
        }
      }
      LiteralEntity::Symbol(_, str_rep) => str_rep,
      LiteralEntity::Infinity(positive) => {
        if positive {
          "Infinity"
        } else {
          "-Infinity"
        }
      }
      LiteralEntity::NaN => "NaN",
      LiteralEntity::Null => "null",
      LiteralEntity::Undefined => "undefined",
    }
  }

  // `None` for unresolvable, `Some(None)` for NaN, `Some(Some(value))` for number
  pub fn to_number(self) -> Option<Option<F64WithEq>> {
    match self {
      LiteralEntity::Number(value, _) => Some(Some(value)),
      LiteralEntity::BigInt(_value) => {
        // TODO: warn: TypeError: Cannot convert a BigInt value to a number
        None
      }
      LiteralEntity::Boolean(value) => Some(Some(if value { 1.0 } else { 0.0 }.into())),
      LiteralEntity::String(value, _) => {
        let value = value.trim();
        Some(if value.is_empty() {
          Some(0.0.into())
        } else if let Ok(value) = value.parse::<f64>() {
          Some(value.into())
        } else {
          None
        })
      }
      LiteralEntity::Null => Some(Some(0.0.into())),
      LiteralEntity::Symbol(_, _) => {
        // TODO: warn: TypeError: Cannot convert a Symbol value to a number
        None
      }
      LiteralEntity::NaN | LiteralEntity::Undefined => Some(None),
      LiteralEntity::Infinity(_) => None,
    }
  }

  fn get_prototype(&self, analyzer: &mut Analyzer<'a>) -> &'a BuiltinPrototype<'a> {
    match self {
      LiteralEntity::String(_, _) => &analyzer.builtins.prototypes.string,
      LiteralEntity::Number(_, _) => &analyzer.builtins.prototypes.number,
      LiteralEntity::BigInt(_) => &analyzer.builtins.prototypes.bigint,
      LiteralEntity::Boolean(_) => &analyzer.builtins.prototypes.boolean,
      LiteralEntity::Symbol(_, _) => &analyzer.builtins.prototypes.symbol,
      LiteralEntity::Infinity(_) => &analyzer.builtins.prototypes.number,
      LiteralEntity::NaN => &analyzer.builtins.prototypes.number,
      LiteralEntity::Null | LiteralEntity::Undefined => {
        unreachable!("Cannot get prototype of null or undefined")
      }
    }
  }

  fn get_known_instance_property(
    &self,
    analyzer: &Analyzer<'a>,
    key: LiteralEntity<'a>,
  ) -> Option<Entity<'a>> {
    match self {
      LiteralEntity::String(value, atom_self) => {
        let LiteralEntity::String(key, atom_key) = key else { return None };
        if key == "length" {
          Some(analyzer.factory.number(value.len() as f64, None))
        } else if let Ok(index) = key.parse::<usize>() {
          Some(
            value
              .get(index..index + 1)
              .map_or(analyzer.factory.undefined, |v| analyzer.factory.string(v)),
          )
        } else {
          None
        }
        .map(|val| analyzer.factory.computed(val, (*atom_self, atom_key)))
      }
      _ => None,
    }
  }

  pub fn strict_eq(self, other: LiteralEntity) -> (bool, Option<MangleConstraint>) {
    // 0.0 === -0.0
    if let (LiteralEntity::Number(l, _), LiteralEntity::Number(r, _)) = (self, other) {
      let eq = if l == 0.0.into() || l == (-0.0).into() {
        r == 0.0.into() || r == (-0.0).into()
      } else {
        l == r
      };
      return (eq, None);
    }

    if let (LiteralEntity::String(l, atom_l), LiteralEntity::String(r, atom_r)) = (self, other) {
      let eq = l == r;
      return (eq, MangleConstraint::equality(eq, atom_l, atom_r));
    }

    (self == other && self != LiteralEntity::NaN, None)
  }

  pub fn with_mangle_atom(self, analyzer: &mut Analyzer<'a>) -> (Entity<'a>, Option<MangleAtom>) {
    match self {
      LiteralEntity::String(value, atom) => {
        let atom = atom.unwrap_or_else(|| analyzer.mangler.new_atom());
        (analyzer.factory.alloc(LiteralEntity::String(value, Some(atom))), Some(atom))
      }
      _ => (analyzer.factory.alloc(self), None),
    }
  }
}
