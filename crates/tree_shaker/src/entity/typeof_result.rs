use bitflags::bitflags;

bitflags! {
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub struct TypeofResult: u8 {
    const _None = 0;
    const _Unknown = 0xFF;
    const _Primitive = Self::String.bits()
      | Self::Number.bits()
      | Self::BigInt.bits()
      | Self::Boolean.bits()
      | Self::Symbol.bits()
      | Self::Undefined.bits();

    const String = 1 << 0;
    const Number = 1 << 1;
    const BigInt = 1 << 2;
    const Boolean = 1 << 3;
    const Symbol = 1 << 4;
    const Undefined = 1 << 5;
    const Object = 1 << 6;
    const Function = 1 << 7;
  }
}

impl TypeofResult {
  pub fn to_string(self) -> Option<&'static str> {
    Some(match self {
      TypeofResult::String => "string",
      TypeofResult::Number => "number",
      TypeofResult::BigInt => "bigint",
      TypeofResult::Boolean => "boolean",
      TypeofResult::Symbol => "symbol",
      TypeofResult::Undefined => "undefined",
      TypeofResult::Object => "object",
      TypeofResult::Function => "function",
      _ => return None,
    })
  }
}
