use std::ops::{Add, AddAssign};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Found {
  True,
  False,
  Unknown,
}

impl Found {
  pub fn may_found(self) -> bool {
    matches!(self, Found::True | Found::Unknown)
  }

  pub fn must_found(self) -> bool {
    matches!(self, Found::True)
  }

  pub fn must_not_found(self) -> bool {
    matches!(self, Found::False)
  }

  pub fn is_unknown(self) -> bool {
    matches!(self, Found::Unknown)
  }

  pub fn known(found: bool) -> Self {
    if found { Found::True } else { Found::False }
  }
}

impl Add for Found {
  type Output = Self;

  fn add(self, rhs: Self) -> Self::Output {
    match (self, rhs) {
      (Found::Unknown, _) | (_, Found::Unknown) => Found::Unknown,
      (Found::False, Found::False) => Found::False,
      _ => Found::True,
    }
  }
}

impl AddAssign for Found {
  fn add_assign(&mut self, rhs: Self) {
    *self = *self + rhs;
  }
}
