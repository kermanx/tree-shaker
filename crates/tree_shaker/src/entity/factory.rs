use crate::{
  consumable::{Consumable, ConsumableTrait, LazyConsumable, OnceConsumable},
  mangling::{AlwaysMangableDep, MangleAtom, MangleConstraint, ManglingDep},
  scope::CfScopeId,
  utils::F64WithEq,
  TreeShakeConfig,
};

use super::{
  arguments::ArgumentsEntity,
  array::ArrayEntity,
  builtin_fn::{BuiltinFnImplementation, ImplementedBuiltinFnEntity},
  computed::ComputedEntity,
  logical_result::LogicalResultEntity,
  never::NeverEntity,
  react_element::ReactElementEntity,
  union::UnionEntity,
  utils::UnionLike,
  Entity, LiteralEntity, ObjectEntity, ObjectPrototype, PrimitiveEntity, PureBuiltinFnEntity,
  UnknownEntity,
};
use oxc::allocator::Allocator;
use oxc::semantic::SymbolId;
use oxc_syntax::operator::LogicalOperator;

use std::{
  cell::{Cell, RefCell},
  fmt::Debug,
};
pub struct EntityFactory<'a> {
  pub allocator: &'a Allocator,
  instance_id_counter: Cell<usize>,

  pub r#true: Entity<'a>,
  pub r#false: Entity<'a>,
  pub nan: Entity<'a>,
  pub null: Entity<'a>,
  pub undefined: Entity<'a>,

  pub never: Entity<'a>,
  pub immutable_unknown: Entity<'a>,

  pub unknown_primitive: Entity<'a>,
  pub unknown_string: Entity<'a>,
  pub unknown_number: Entity<'a>,
  pub unknown_bigint: Entity<'a>,
  pub unknown_boolean: Entity<'a>,
  pub unknown_symbol: Entity<'a>,

  pub pure_fn_returns_unknown: Entity<'a>,
  pub pure_fn_returns_string: Entity<'a>,
  pub pure_fn_returns_number: Entity<'a>,
  pub pure_fn_returns_bigint: Entity<'a>,
  pub pure_fn_returns_boolean: Entity<'a>,
  pub pure_fn_returns_symbol: Entity<'a>,
  pub pure_fn_returns_null: Entity<'a>,
  pub pure_fn_returns_undefined: Entity<'a>,

  pub empty_arguments: Entity<'a>,
  pub unmatched_prototype_property: Entity<'a>,

  pub empty_consumable: Consumable<'a>,
  pub consumed_lazy_consumable: LazyConsumable<'a>,
}

impl<'a> EntityFactory<'a> {
  pub fn new(allocator: &'a Allocator, config: &TreeShakeConfig) -> EntityFactory<'a> {
    let r#true = allocator.alloc(LiteralEntity::Boolean(true));
    let r#false = allocator.alloc(LiteralEntity::Boolean(false));
    let nan = allocator.alloc(LiteralEntity::NaN);
    let null = allocator.alloc(LiteralEntity::Null);
    let undefined = allocator.alloc(LiteralEntity::Undefined);

    let never = allocator.alloc(NeverEntity);
    let immutable_unknown = allocator.alloc(UnknownEntity::new());
    let unknown_primitive = allocator.alloc(PrimitiveEntity::Mixed);
    let unknown_string = allocator.alloc(PrimitiveEntity::String);
    let unknown_number = allocator.alloc(PrimitiveEntity::Number);
    let unknown_bigint = allocator.alloc(PrimitiveEntity::BigInt);
    let unknown_boolean = allocator.alloc(PrimitiveEntity::Boolean);
    let unknown_symbol = allocator.alloc(PrimitiveEntity::Symbol);

    let pure_fn_returns_unknown = allocator.alloc(PureBuiltinFnEntity::new(|f| f.unknown()));

    let pure_fn_returns_string = allocator.alloc(PureBuiltinFnEntity::new(|f| f.unknown_string));
    let pure_fn_returns_number = allocator.alloc(PureBuiltinFnEntity::new(|f| f.unknown_number));
    let pure_fn_returns_bigint = allocator.alloc(PureBuiltinFnEntity::new(|f| f.unknown_bigint));
    let pure_fn_returns_boolean = allocator.alloc(PureBuiltinFnEntity::new(|f| f.unknown_boolean));
    let pure_fn_returns_symbol = allocator.alloc(PureBuiltinFnEntity::new(|f| f.unknown_symbol));
    let pure_fn_returns_null = allocator.alloc(PureBuiltinFnEntity::new(|f| f.null));
    let pure_fn_returns_undefined = allocator.alloc(PureBuiltinFnEntity::new(|f| f.undefined));

    let empty_arguments = allocator.alloc(ArgumentsEntity::default());
    let unmatched_prototype_property: Entity<'a> =
      if config.unmatched_prototype_property_as_undefined { undefined } else { immutable_unknown };

    let empty_consumable = Consumable(allocator.alloc(()));
    let consumed_lazy_consumable = LazyConsumable(allocator.alloc(RefCell::new(None)));

    EntityFactory {
      allocator,
      instance_id_counter: Cell::new(0),

      r#true,
      r#false,
      nan,
      null,
      undefined,

      never,
      immutable_unknown,

      unknown_primitive,
      unknown_string,
      unknown_number,
      unknown_bigint,
      unknown_boolean,
      unknown_symbol,

      pure_fn_returns_unknown,
      pure_fn_returns_string,
      pure_fn_returns_number,
      pure_fn_returns_bigint,
      pure_fn_returns_boolean,
      pure_fn_returns_symbol,
      pure_fn_returns_null,
      pure_fn_returns_undefined,

      empty_arguments,
      unmatched_prototype_property,

      empty_consumable,
      consumed_lazy_consumable,
    }
  }

  pub fn alloc<T>(&self, val: T) -> &'a mut T {
    self.allocator.alloc(val)
  }

  pub fn alloc_instance_id(&self) -> usize {
    let id = self.instance_id_counter.get();
    self.instance_id_counter.set(id + 1);
    id
  }

  pub fn builtin_object(
    &self,
    object_id: SymbolId,
    prototype: ObjectPrototype<'a>,
    consumable: bool,
  ) -> &'a mut ObjectEntity<'a> {
    self.alloc(ObjectEntity {
      consumable,
      consumed: Cell::new(false),
      consumed_as_prototype: Cell::new(false),
      cf_scope: CfScopeId::new(0),
      object_id,
      string_keyed: Default::default(),
      unknown_keyed: Default::default(),
      rest: Default::default(),
      prototype: Cell::new(prototype),
      mangling_group: None,
    })
  }

  pub fn arguments(&self, arguments: Vec<(bool, Entity<'a>)>) -> Entity<'a> {
    self.alloc(ArgumentsEntity { consumed: Cell::new(false), arguments })
  }

  pub fn array(&self, cf_scope: CfScopeId, object_id: SymbolId) -> &'a mut ArrayEntity<'a> {
    self.alloc(ArrayEntity {
      consumed: Cell::new(false),
      deps: Default::default(),
      cf_scope,
      object_id,
      elements: RefCell::new(Vec::new()),
      rest: RefCell::new(Vec::new()),
    })
  }

  pub fn implemented_builtin_fn<F: BuiltinFnImplementation<'a> + 'a>(
    &self,
    _name: &'static str,
    implementation: F,
  ) -> Entity<'a> {
    self.alloc(ImplementedBuiltinFnEntity {
      #[cfg(feature = "flame")]
      name: _name,
      implementation,
      object: None,
    })
  }

  pub fn computed<T: ConsumableTrait<'a> + Copy + 'a>(
    &self,
    val: Entity<'a>,
    dep: T,
  ) -> Entity<'a> {
    self.alloc(ComputedEntity { val, dep, consumed: Cell::new(false) })
  }

  pub fn consumable_no_once(&self, dep: impl ConsumableTrait<'a> + 'a) -> Consumable<'a> {
    Consumable(self.alloc(dep))
  }

  pub fn consumable_once(&self, dep: impl ConsumableTrait<'a> + 'a) -> Consumable<'a> {
    self.consumable_no_once(OnceConsumable::new(dep))
  }

  pub fn consumable(&self, dep: impl ConsumableTrait<'a> + 'a) -> Consumable<'a> {
    self.consumable_once(dep)
  }

  pub fn optional_computed(
    &self,
    val: Entity<'a>,
    dep: Option<impl ConsumableTrait<'a> + Copy + 'a>,
  ) -> Entity<'a> {
    match dep {
      Some(dep) => self.computed(val, dep),
      None => val,
    }
  }

  pub fn string(&self, value: &'a str) -> Entity<'a> {
    self.alloc(LiteralEntity::String(value, None))
  }

  pub fn mangable_string(&self, value: &'a str, atom: MangleAtom) -> Entity<'a> {
    self.alloc(LiteralEntity::String(value, Some(atom)))
  }

  pub fn number(&self, value: impl Into<F64WithEq>, str_rep: Option<&'a str>) -> Entity<'a> {
    self.alloc(LiteralEntity::Number(value.into(), str_rep))
  }
  pub fn big_int(&self, value: &'a str) -> Entity<'a> {
    self.alloc(LiteralEntity::BigInt(value))
  }

  pub fn boolean(&self, value: bool) -> Entity<'a> {
    if value {
      self.r#true
    } else {
      self.r#false
    }
  }
  pub fn boolean_maybe_unknown(&self, value: Option<bool>) -> Entity<'a> {
    if let Some(value) = value {
      self.boolean(value)
    } else {
      self.unknown_boolean
    }
  }

  pub fn infinity(&self, positivie: bool) -> Entity<'a> {
    self.alloc(LiteralEntity::Infinity(positivie))
  }

  pub fn symbol(&self, id: SymbolId, str_rep: &'a str) -> Entity<'a> {
    self.alloc(LiteralEntity::Symbol(id, str_rep))
  }

  /// Only used when (maybe_left, maybe_right) == (true, true)
  pub fn logical_result(
    &self,
    left: Entity<'a>,
    right: Entity<'a>,
    operator: LogicalOperator,
  ) -> &'a mut LogicalResultEntity<'a> {
    self.alloc(LogicalResultEntity {
      value: self.union((left, right)),
      is_coalesce: operator == LogicalOperator::Coalesce,
      result: match operator {
        LogicalOperator::Or => match right.test_truthy() {
          Some(true) => Some(true),
          _ => None,
        },
        LogicalOperator::And => match right.test_truthy() {
          Some(false) => Some(false),
          _ => None,
        },
        LogicalOperator::Coalesce => match right.test_nullish() {
          Some(true) => Some(true),
          _ => None,
        },
      },
    })
  }

  pub fn try_union<V: UnionLike<'a, Entity<'a>> + Debug + 'a>(
    &self,
    values: V,
  ) -> Option<Entity<'a>> {
    match values.len() {
      0 => None,
      1 => Some(values.iter().next().unwrap()),
      _ => Some(self.alloc(UnionEntity {
        values,
        consumed: Cell::new(false),
        phantom: std::marker::PhantomData,
      })),
    }
  }

  pub fn union<V: UnionLike<'a, Entity<'a>> + Debug + 'a>(&self, values: V) -> Entity<'a> {
    self.try_union(values).unwrap()
  }

  pub fn optional_union(
    &self,
    entity: Entity<'a>,
    entity_option: Option<Entity<'a>>,
  ) -> Entity<'a> {
    if let Some(entity_option) = entity_option {
      self.union((entity, entity_option))
    } else {
      entity
    }
  }

  pub fn computed_union<T: ConsumableTrait<'a> + Copy + 'a>(
    &self,
    values: Vec<Entity<'a>>,
    dep: T,
  ) -> Entity<'a> {
    self.computed(self.union(values), dep)
  }

  pub fn unknown(&self) -> Entity<'a> {
    self.immutable_unknown
  }

  pub fn computed_unknown(&self, dep: impl ConsumableTrait<'a> + Copy + 'a) -> Entity<'a> {
    self.computed(self.immutable_unknown, dep)
  }

  pub fn new_lazy_consumable(&self, consumable: Consumable<'a>) -> LazyConsumable<'a> {
    LazyConsumable(self.alloc(RefCell::new(Some(vec![consumable]))))
  }

  pub fn react_element(
    &self,
    tag: Entity<'a>,
    props: Entity<'a>,
  ) -> &'a mut ReactElementEntity<'a> {
    self.alloc(ReactElementEntity {
      consumed: Cell::new(false),
      tag,
      props,
      deps: RefCell::new(vec![]),
    })
  }

  pub fn mangable(
    &self,
    val: Entity<'a>,
    deps: (Entity<'a>, Entity<'a>),
    constraint: &'a MangleConstraint,
  ) -> Entity<'a> {
    self.computed(val, ManglingDep { deps, constraint })
  }

  pub fn always_mangable_dep(&self, dep: Entity<'a>) -> Consumable<'a> {
    self.consumable(AlwaysMangableDep { dep })
  }
}

macro_rules! unknown_entity_ctors {
  ($($name:ident -> $var:ident,)*) => {
    $(
      #[allow(unused)]
      pub fn $name<T: ConsumableTrait<'a> + Copy + 'a>(&self, dep: T) -> Entity<'a> {
        self.computed(self.$var, dep)
      }
    )*
  };
}

impl<'a> EntityFactory<'a> {
  unknown_entity_ctors! {
    computed_unknown_primitive -> unknown_primitive,
    computed_unknown_boolean -> unknown_boolean,
    computed_unknown_number -> unknown_number,
    computed_unknown_string -> unknown_string,
    computed_unknown_bigint -> unknown_bigint,
    computed_unknown_symbol -> unknown_symbol,
  }
}
