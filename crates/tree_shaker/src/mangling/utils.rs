/// Adapted from https://github.com/oxc-project/oxc/blob/774babb7f2c9a36a12804d76c4c9b6b5684569bb/crates/oxc_mangler/src/lib.rs
use rustc_hash::FxHashSet;

use crate::entity::LiteralEntity;

const BASE54_CHARS: &[u8; 64] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ$_0123456789";

/// Get the shortest mangled name for a given n.
/// Code adapted from [terser](https://github.com/terser/terser/blob/8b966d687395ab493d2c6286cc9dd38650324c11/lib/scope.js#L1041-L1051)
pub fn get_mangled_name(n: usize) -> String {
  let mut num = n;
  // Base 54 at first because these are the usable first characters in JavaScript identifiers
  // <https://tc39.es/ecma262/#prod-IdentifierStart>
  let base = 54usize;
  let mut ret = String::new();
  ret.push(BASE54_CHARS[num % base] as char);
  num /= base;
  // Base 64 for the rest because after the first character we can also use 0-9 too
  // <https://tc39.es/ecma262/#prod-IdentifierPart>
  let base = 64usize;
  while num > 0 {
    num -= 1;
    ret.push(BASE54_CHARS[num % base] as char);
    num /= base;
  }
  ret
}

pub fn is_literal_mangable(literals: &FxHashSet<LiteralEntity<'_>>) -> bool {
  for key_literal in literals {
    if let LiteralEntity::String(_, key_atom) = key_literal {
      if key_atom.is_none() {
        return false;
      }
    }
  }
  true
}
