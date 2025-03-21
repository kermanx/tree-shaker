use crate::{
  analyzer::Analyzer,
  entity::{Entity, LiteralEntity, TypeofResult, ValueTrait},
  mangling::MangleConstraint,
};
use oxc::ast::ast::{BinaryOperator, UpdateOperator};
use oxc_ecmascript::ToInt32;

impl<'a> Analyzer<'a> {
  pub fn op_loose_eq(
    &self,
    lhs: Entity<'a>,
    rhs: Entity<'a>,
  ) -> (Option<bool>, Option<MangleConstraint<'a>>) {
    if let (Some(true), m) = self.op_strict_eq(lhs, rhs) {
      return (Some(true), m);
    }

    if lhs.test_nullish() == Some(true) && rhs.test_nullish() == Some(true) {
      return (Some(true), None);
    }

    (None, None)
  }

  pub fn op_loose_neq(
    &self,
    lhs: Entity<'a>,
    rhs: Entity<'a>,
  ) -> (Option<bool>, Option<MangleConstraint<'a>>) {
    if let (Some(eq), m) = self.op_loose_eq(lhs, rhs) {
      return (Some(!eq), m.map(|m| m.negate_equality(self.allocator)));
    }

    let lhs_lit = lhs.get_literal(self);
    let rhs_lit = rhs.get_literal(self);
    if let (Some(lhs_lit), Some(rhs_lit)) = (lhs_lit, rhs_lit) {
      if lhs_lit.test_typeof() == rhs_lit.test_typeof() {
        let (eq, m) = lhs_lit.strict_eq(rhs_lit);
        return (Some(!eq), m.map(|m| m.negate_equality(self.allocator)));
      }
    }

    (None, None)
  }

  pub fn op_strict_eq(
    &self,
    lhs: Entity<'a>,
    rhs: Entity<'a>,
  ) -> (Option<bool>, Option<MangleConstraint<'a>>) {
    // TODO: Find another way to do this
    // if Entity::ptr_eq(lhs, rhs) {
    //   return Some(true);
    // }

    let lhs_t = lhs.test_typeof();
    let rhs_t = rhs.test_typeof();
    if lhs_t & rhs_t == TypeofResult::_None {
      return (Some(false), None);
    }

    let lhs_lit = lhs.get_to_literals(self);
    let rhs_lit = rhs.get_to_literals(self);
    if let (Some(lhs_lit), Some(rhs_lit)) = (lhs_lit, rhs_lit) {
      if lhs_lit.len() == 1 && rhs_lit.len() == 1 {
        let lhs_lit = *lhs_lit.iter().next().unwrap();
        let rhs_lit = *rhs_lit.iter().next().unwrap();
        let (eq, m) = lhs_lit.strict_eq(rhs_lit);
        return (Some(eq), m);
      }

      let mut constraints = Some(self.allocator.alloc(self.factory.vec()));
      let mut all_neq = true;
      'check: for l in &lhs_lit {
        for r in &rhs_lit {
          let (eq, mc) = l.strict_eq(*r);
          all_neq &= !eq;
          if let Some(mc) = mc {
            if let Some(constraints) = &mut constraints {
              constraints.push(mc);
            } else {
              constraints = None;
            }
          } else if !all_neq {
            break 'check;
          }
        }
      }

      return (
        if all_neq { Some(false) } else { None },
        constraints.map(|m| MangleConstraint::Multiple(m)),
      );
    }

    (None, None)
  }

  pub fn op_strict_neq(
    &self,
    lhs: Entity<'a>,
    rhs: Entity<'a>,
  ) -> (Option<bool>, Option<MangleConstraint<'a>>) {
    let (eq, m) = self.op_strict_eq(lhs, rhs);
    (eq.map(|v| !v), m.map(|m| m.negate_equality(self.allocator)))
  }

  pub fn op_lt(&self, lhs: Entity<'a>, rhs: Entity<'a>, eq: bool) -> Option<bool> {
    fn literal_lt(lhs: &LiteralEntity, rhs: &LiteralEntity, eq: bool) -> Option<bool> {
      match (lhs, rhs) {
        (LiteralEntity::Number(l, _), LiteralEntity::Number(r, _)) => {
          Some(if eq { l.0 <= r.0 } else { l.0 < r.0 })
        }
        (LiteralEntity::String(l, _), LiteralEntity::String(r, _)) => {
          Some(if eq { l <= r } else { l < r })
        }
        (LiteralEntity::BigInt(_), LiteralEntity::BigInt(_))
        | (LiteralEntity::BigInt(_), LiteralEntity::String(_, _))
        | (LiteralEntity::String(_, _), LiteralEntity::BigInt(_)) => None,
        (lhs, rhs) => {
          let lhs = lhs.to_number();
          let rhs = rhs.to_number();
          match (lhs, rhs) {
            (None, _) | (_, None) => None,
            (Some(None), _) | (_, Some(None)) => Some(false),
            (Some(Some(l)), Some(Some(r))) => Some(if eq { l.0 <= r.0 } else { l.0 < r.0 }),
          }
        }
      }
    }

    if let (Some(lhs), Some(rhs)) = (lhs.get_to_literals(self), rhs.get_to_literals(self)) {
      let mut result = None;
      for lhs in lhs.iter() {
        for rhs in rhs.iter() {
          if let Some(v) = literal_lt(lhs, rhs, eq) {
            if let Some(result) = result {
              if result != v {
                return None;
              }
            } else {
              result = Some(v);
            }
          } else {
            return None;
          }
        }
      }
      assert!(result.is_some());
      result
    } else {
      None
    }
  }

  pub fn op_gt(&self, lhs: Entity<'a>, rhs: Entity<'a>, eq: bool) -> Option<bool> {
    self.op_lt(rhs, lhs, eq)
  }

  pub fn op_instanceof(&self, lhs: Entity<'a>, _rhs: Entity<'a>) -> Option<bool> {
    if (TypeofResult::String
      | TypeofResult::Number
      | TypeofResult::BigInt
      | TypeofResult::Boolean
      | TypeofResult::Symbol
      | TypeofResult::Undefined)
      .contains(lhs.test_typeof())
      || lhs.test_nullish() == Some(true)
    {
      Some(false)
    } else {
      None
    }
  }

  pub fn op_add(&self, lhs: Entity<'a>, rhs: Entity<'a>) -> Entity<'a> {
    let lhs_t = lhs.test_typeof();
    let rhs_t = rhs.test_typeof();
    let lhs_lit = lhs.get_literal(self);
    let rhs_lit = rhs.get_literal(self);

    let mut values = self.factory.vec();

    let may_convert_to_num = TypeofResult::Number
      | TypeofResult::Boolean
      | TypeofResult::Undefined
      | TypeofResult::Object
      | TypeofResult::Function;
    let must_not_convert_to_str =
      TypeofResult::Number | TypeofResult::Boolean | TypeofResult::Undefined | TypeofResult::BigInt;

    if lhs_t.intersects(may_convert_to_num) && rhs_t.intersects(may_convert_to_num) {
      // Possibly number
      match (lhs_lit.and_then(|v| v.to_number()), rhs_lit.and_then(|v| v.to_number())) {
        (Some(l), Some(r)) => match (l, r) {
          (Some(l), Some(r)) => {
            let val = l.0 + r.0;
            values.push(self.factory.number(val, None));
          }
          _ => {
            values.push(self.factory.nan);
          }
        },
        _ => {
          values.push(self.factory.unknown_number);
        }
      }
    }
    if lhs_t.contains(TypeofResult::BigInt) && rhs_t.contains(TypeofResult::BigInt) {
      // Possibly bigint
      values.push(self.factory.unknown_bigint);
    }
    if !lhs_t.difference(must_not_convert_to_str).is_empty()
      || !rhs_t.difference(must_not_convert_to_str).is_empty()
    {
      let lhs_str = lhs.get_to_string(self);
      let rhs_str = rhs.get_to_string(self);

      let lhs_str_lit = lhs_str.get_literal(self);
      let rhs_str_lit = rhs_str.get_literal(self);

      match (lhs_str_lit, rhs_str_lit) {
        (Some(LiteralEntity::String(l, _)), Some(LiteralEntity::String(r, _))) => {
          let val = l.to_string() + r;
          values.push(self.factory.string(self.allocator.alloc_str(&val)));
        }
        _ => {
          values.push(self.factory.unknown_string);
        }
      }
    }

    if values.is_empty() {
      // TODO: throw warning
      self.factory.computed_unknown((lhs, rhs))
    } else {
      self.factory.computed_union(values, (lhs, rhs))
    }
  }

  fn op_numeric(
    &self,
    lhs: Entity<'a>,
    rhs: Entity<'a>,
    calc: impl FnOnce(f64, f64) -> Entity<'a>,
  ) -> Entity<'a> {
    self.factory.computed(
      if let (Some(l), Some(r)) = (lhs.get_literal(self), rhs.get_literal(self)) {
        match (l, r) {
          (LiteralEntity::Number(l, _), LiteralEntity::Number(r, _)) => calc(l.0, r.0),
          (LiteralEntity::NaN, _) | (_, LiteralEntity::NaN) => self.factory.nan,
          _ => self.factory.unknown_primitive,
        }
      } else {
        self.factory.unknown_primitive
      },
      (lhs, rhs),
    )
  }

  pub fn op_update(&self, input: Entity<'a>, operator: UpdateOperator) -> Entity<'a> {
    let apply_update = |v: f64| {
      let val = match operator {
        UpdateOperator::Increment => v + 1.0,
        UpdateOperator::Decrement => v - 1.0,
      };
      self.factory.number(val, None)
    };

    if let Some(num) = input.get_literal(self).and_then(|lit| lit.to_number()) {
      return self.factory.computed(
        match num {
          Some(num) => apply_update(num.0),
          None => self.factory.nan,
        },
        input,
      );
    }

    let input_t = input.test_typeof();

    let mut values = self.factory.vec();
    if input_t.contains(TypeofResult::BigInt) {
      values.push(self.factory.unknown_bigint);
    }
    if input_t.contains(TypeofResult::Number) {
      values.push(self.factory.unknown_number);
    }

    if values.is_empty() {
      self.factory.computed_unknown(input)
    } else {
      self.factory.computed_union(values, input)
    }
  }

  pub fn op_binary(
    &self,
    operator: BinaryOperator,
    lhs: Entity<'a>,
    rhs: Entity<'a>,
  ) -> Entity<'a> {
    let factory = self.factory;

    let to_result =
      |result: Option<bool>| factory.computed(factory.boolean_maybe_unknown(result), (lhs, rhs));

    let to_eq_result =
      |(equality, mangle_constraint): (Option<bool>, Option<MangleConstraint<'a>>)| {
        if let Some(mangle_constraint) = mangle_constraint {
          factory.mangable(factory.boolean_maybe_unknown(equality), (lhs, rhs), mangle_constraint)
        } else {
          to_result(equality)
        }
      };

    match operator {
      BinaryOperator::Equality => to_eq_result(self.op_loose_eq(lhs, rhs)),
      BinaryOperator::Inequality => to_eq_result(self.op_loose_neq(lhs, rhs)),
      BinaryOperator::StrictEquality => to_eq_result(self.op_strict_eq(lhs, rhs)),
      BinaryOperator::StrictInequality => to_eq_result(self.op_strict_neq(lhs, rhs)),
      BinaryOperator::LessThan => to_result(self.op_lt(lhs, rhs, false)),
      BinaryOperator::LessEqualThan => to_result(self.op_lt(lhs, rhs, true)),
      BinaryOperator::GreaterThan => to_result(self.op_gt(lhs, rhs, false)),
      BinaryOperator::GreaterEqualThan => to_result(self.op_gt(lhs, rhs, true)),
      BinaryOperator::Addition => self.op_add(lhs, rhs),

      BinaryOperator::Subtraction
      | BinaryOperator::Multiplication
      | BinaryOperator::Division
      | BinaryOperator::Remainder
      | BinaryOperator::Exponential => self.op_numeric(lhs, rhs, |l, r| {
        let value = match operator {
          BinaryOperator::Subtraction => l - r,
          BinaryOperator::Multiplication => l * r,
          BinaryOperator::Division => l / r,
          BinaryOperator::Remainder => {
            if r == 0.0 {
              f64::NAN
            } else {
              l % r
            }
          }
          BinaryOperator::Exponential => l.powf(r),
          _ => unreachable!(),
        };
        if value.is_nan() { factory.nan } else { factory.number(value, None) }
      }),

      BinaryOperator::ShiftLeft
      | BinaryOperator::ShiftRight
      | BinaryOperator::ShiftRightZeroFill => {
        self.op_numeric(lhs, rhs, |l, r| {
          // https://github.com/oxc-project/oxc/blob/main/crates/oxc_ecmascript/src/constant_evaluation/mod.rs
          if l.fract() != 0.0 || r.fract() != 0.0 || !(0.0..32.0).contains(&r) {
            return factory.unknown_number;
          }
          let bits = l.to_int_32();
          let right_val_int = r as u32;
          let value = match operator {
            BinaryOperator::ShiftLeft => f64::from(bits.wrapping_shl(right_val_int)),
            BinaryOperator::ShiftRight => f64::from(bits.wrapping_shr(right_val_int)),
            BinaryOperator::ShiftRightZeroFill => {
              // JavaScript always treats the result of >>> as unsigned.
              // We must force Rust to do the same here.
              let bits = bits as u32;
              let res = bits.wrapping_shr(right_val_int);
              f64::from(res)
            }
            _ => unreachable!(),
          };
          factory.number(value, None)
        })
      }

      BinaryOperator::BitwiseOR | BinaryOperator::BitwiseXOR | BinaryOperator::BitwiseAnd => self
        .op_numeric(lhs, rhs, |l, r| {
          let l = l.to_int_32();
          let r = r.to_int_32();
          let value = match operator {
            BinaryOperator::BitwiseOR => l | r,
            BinaryOperator::BitwiseXOR => l ^ r,
            BinaryOperator::BitwiseAnd => l & r,
            _ => unreachable!(),
          };
          factory.number(f64::from(value), None)
        }),

      BinaryOperator::In => factory.computed_unknown_boolean((lhs, rhs)),
      BinaryOperator::Instanceof => to_result(self.op_instanceof(lhs, rhs)),
    }
  }
}
