use super::{BuiltinPrototype, object::create_object_prototype};
use crate::{
  entity::{EntityFactory, ObjectId},
  init_prototype,
};

pub fn create_function_prototype<'a>(factory: &EntityFactory<'a>) -> BuiltinPrototype<'a> {
  init_prototype!("Function", create_object_prototype(factory), {
    "apply" => factory.implemented_builtin_fn("Function::apply", |analyzer, dep, this, args| {
      let mut args = args.destruct_as_array(analyzer, dep, 2, false).0;
      let args_arg = {
        let arg = args.pop().unwrap();
        let cf_scope = analyzer.scoping.cf.current_id();
        // This can be any value
        let arguments_object_id = ObjectId::from_usize(0);
        match arg.test_is_undefined() {
          Some(true) => analyzer.factory.array(cf_scope, arguments_object_id).into(),
          Some(false) => arg,
          None => analyzer.factory.union((
            arg,
            analyzer.factory.array(cf_scope, arguments_object_id).into(),
          )),
        }
      };
      let this_arg = args.pop().unwrap();
      this.call(analyzer, dep, this_arg, args_arg)
    }),
    "call" => factory.implemented_builtin_fn("Function::call", |analyzer, dep, this, args| {
      let (this_arg, args_arg, _deps) = args.destruct_as_array(analyzer, dep, 1, true);
      this.call(analyzer, dep, this_arg[0], args_arg.unwrap())
    }),
    "bind" => factory.pure_fn_returns_unknown,
    "length" => factory.unknown_number,
    "arguments" => factory.immutable_unknown,
    "caller" => factory.immutable_unknown,
    "name" => factory.unknown_string,
    "prototype" => factory.immutable_unknown,
  })
}
