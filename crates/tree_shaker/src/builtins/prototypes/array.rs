use super::{BuiltinPrototype, object::create_object_prototype};
use crate::{analyzer::Factory, init_prototype};

pub fn create_array_prototype<'a>(factory: &Factory<'a>) -> BuiltinPrototype<'a> {
  init_prototype!("Array", create_object_prototype(factory), {
    "at" => factory.unknown,
    "concat" => factory.unknown /*pure_fn_returns_array*/,
    "copyWithin" => factory.pure_fn_returns_unknown /* mutates_self */,
    "entries" => factory.unknown /*pure_fn_returns_array*/,
    "every" => factory.pure_fn_returns_boolean,
    "fill" => factory.pure_fn_returns_unknown /* mutates_self */,
    "filter" => factory.unknown /*pure_fn_returns_array*/,
    "find" => factory.pure_fn_returns_unknown,
    "findIndex" => factory.pure_fn_returns_number,
    "findLast" => factory.pure_fn_returns_unknown,
    "findLastIndex" => factory.pure_fn_returns_number,
    "flat" => factory.unknown /*pure_fn_returns_array*/,
    "flatMap" => factory.unknown /*pure_fn_returns_array*/,
    "forEach" => factory.pure_fn_returns_unknown,
    "includes" => factory.pure_fn_returns_boolean,
    "indexOf" => factory.pure_fn_returns_number,
    "join" => factory.pure_fn_returns_string,
    "keys" => factory.pure_fn_returns_unknown,
    "lastIndexOf" => factory.pure_fn_returns_number,
    "map" => factory.unknown /*pure_fn_returns_array*/,
    "pop" => factory.pure_fn_returns_unknown /* mutates_self */,
    "push" => factory.pure_fn_returns_number /* mutates_self */,
    "reduce" => factory.pure_fn_returns_unknown,
    "reduceRight" => factory.pure_fn_returns_unknown,
    "reverse" => factory.pure_fn_returns_unknown /* mutates_self */,
    "shift" => factory.pure_fn_returns_unknown /* mutates_self */,
    "slice" => factory.unknown /*pure_fn_returns_array*/,
    "some" => factory.pure_fn_returns_boolean,
    "sort" => factory.pure_fn_returns_unknown /* mutates_self */,
    "splice" => factory.unknown /*pure_fn_returns_array*/ /* mutates_self */,
    "toReversed" => factory.unknown /*pure_fn_returns_array*/,
    "toSorted" => factory.unknown /*pure_fn_returns_array*/,
    "toSpliced" => factory.unknown /*pure_fn_returns_array*/,
    "unshift" => factory.pure_fn_returns_number /* mutates_self */,
    "values" => factory.pure_fn_returns_unknown,
    "with" => factory.unknown,
  })
}
