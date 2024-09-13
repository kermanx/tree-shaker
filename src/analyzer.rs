use crate::{
  ast::{AstType2, DeclarationKind},
  builtins::Builtins,
  data::{get_node_ptr, ExtraData, ReferredNodes, StatementVecData},
  entity::{
    dep::{EntityDep, EntityDepNode},
    entity::Entity,
    forwarded::ForwardedEntity,
    label::LabelEntity,
    operations::EntityOpHost,
    union::UnionEntity,
    unknown::UnknownEntity,
  },
  scope::ScopeContext,
};
use oxc::{
  allocator::Allocator,
  ast::ast::Program,
  semantic::{Semantic, SymbolId},
  span::GetSpan,
};
use rustc_hash::FxHashMap;
use std::mem;

pub struct Analyzer<'a> {
  pub allocator: &'a Allocator,
  pub sematic: Semantic<'a>,
  pub data: ExtraData<'a>,
  pub referred_nodes: ReferredNodes<'a>,
  pub named_exports: Vec<SymbolId>,
  pub default_export: Option<Entity<'a>>,
  pub symbol_decls: FxHashMap<SymbolId, (DeclarationKind, usize, EntityDep<'a>)>,
  pub scope_context: ScopeContext<'a>,
  pub pending_labels: Vec<LabelEntity<'a>>,
  pub builtins: Builtins<'a>,
  pub entity_op: EntityOpHost<'a>,
}

impl<'a> Analyzer<'a> {
  pub fn new(allocator: &'a Allocator, sematic: Semantic<'a>) -> Self {
    Analyzer {
      allocator,
      sematic,
      data: Default::default(),
      referred_nodes: Default::default(),
      named_exports: Vec::new(),
      default_export: None,
      symbol_decls: Default::default(),
      scope_context: ScopeContext::new(),
      pending_labels: Vec::new(),
      builtins: Builtins::new(),
      entity_op: EntityOpHost::new(allocator),
    }
  }

  pub fn exec_program(&mut self, node: &'a Program<'a>) {
    let data = self.load_data::<StatementVecData>(AstType2::Program, node);
    self.exec_statement_vec(data, &node.body);

    debug_assert_eq!(self.scope_context.function_scopes.len(), 1);
    debug_assert_eq!(self.scope_context.variable_scopes.len(), 1);
    debug_assert_eq!(self.scope_context.cf_scopes.len(), 1);

    // Consume exports
    self.default_export.take().map(|entity| entity.consume_as_unknown(self));
    for symbol in self.named_exports.clone() {
      let entity = self.read_symbol(&symbol).clone();
      entity.consume_as_unknown(self);
    }
  }
}

impl<'a> Analyzer<'a> {
  pub fn set_data<T>(&mut self, ast_type: AstType2, node: &'a T, data: impl Default + 'a) {
    let key = (ast_type, get_node_ptr(node));
    self.data.insert(key, unsafe { mem::transmute(Box::new(data)) });
  }

  pub fn load_data<D: Default + 'a>(
    &mut self,
    ast_type: AstType2,
    node: &'a impl GetSpan,
  ) -> &'a mut D {
    let key = (ast_type, get_node_ptr(node));
    let boxed =
      self.data.entry(key).or_insert_with(|| unsafe { mem::transmute(Box::new(D::default())) });
    unsafe { mem::transmute(boxed.as_mut()) }
  }
}

impl<'a> Analyzer<'a> {
  pub fn declare_symbol(
    &mut self,
    symbol: SymbolId,
    dep: EntityDep<'a>,
    exporting: bool,
    kind: DeclarationKind,
    value: Option<Entity<'a>>,
  ) {
    if exporting {
      self.named_exports.push(symbol);
    }
    let (scope_index, scope) = if kind.is_var() {
      let index = self.function_scope().variable_scope_index;
      (index, self.scope_context.variable_scopes.get_mut(index).unwrap())
    } else {
      (self.scope_context.variable_scopes.len() - 1, self.variable_scope_mut())
    };
    scope.declare(kind, symbol, value);
    self.symbol_decls.insert(symbol, (kind, scope_index, dep));
  }

  pub fn init_symbol(&mut self, symbol: SymbolId, value: Entity<'a>) {
    let scope_index = self.symbol_decls.get(&symbol).unwrap().1;
    let scope = self.scope_context.variable_scopes.get_mut(scope_index).unwrap();
    scope.init(symbol, value);
  }

  pub fn new_entity_dep(&self, node: EntityDepNode<'a>) -> EntityDep<'a> {
    EntityDep { node, scope_path: self.variable_scope_path() }
  }

  pub fn read_symbol(&mut self, symbol: &SymbolId) -> Entity<'a> {
    let (_, scope_index, _) = self.symbol_decls.get(symbol).unwrap();
    let variable_scope = &self.scope_context.variable_scopes[*scope_index];
    let cf_scope_index = variable_scope.cf_scope_index;
    let val = self.scope_context.variable_scopes[*scope_index].read(symbol).1;
    self.mark_exhaustive_read(&val, *symbol, cf_scope_index);
    val
  }

  pub fn write_symbol(&mut self, symbol: &SymbolId, new_val: Entity<'a>) {
    let (kind, scope_index, dep) = self.symbol_decls.get(symbol).unwrap().clone();
    if kind.is_const() {
      // TODO: throw warning
    }
    let variable_scope = &self.scope_context.variable_scopes[scope_index];
    let cf_scope_index = variable_scope.cf_scope_index;
    let (is_consumed_exhaustively, old_val) = variable_scope.read(symbol);
    if is_consumed_exhaustively {
      new_val.consume_as_unknown(self);
    } else {
      let entity_to_set = if self.mark_exhaustive_write(&old_val, symbol.clone(), cf_scope_index) {
        old_val.consume_as_unknown(self);
        new_val.consume_as_unknown(self);
        (true, UnknownEntity::new_unknown())
      } else {
        let indeterminate = self.is_relatively_indeterminate(cf_scope_index);
        (
          false,
          ForwardedEntity::new(
            if indeterminate { UnionEntity::new(vec![old_val.clone(), new_val]) } else { new_val },
            dep,
          ),
        )
      };
      self.scope_context.variable_scopes[scope_index].write(*symbol, entity_to_set);
    }
  }

  pub fn refer_dep(&mut self, dep: &EntityDep<'a>) {
    self.referred_nodes.insert(dep.node);

    let mut diff = false;
    for (i, scope) in self.scope_context.variable_scopes.iter_mut().enumerate() {
      if diff || dep.scope_path.get(i) != Some(&scope.id) {
        diff = true;
        scope.has_effect = true;
      }
    }
  }

  pub fn refer_global_dep(&mut self) {
    for scope in self.scope_context.variable_scopes.iter_mut() {
      scope.has_effect = true;
    }
  }
}
