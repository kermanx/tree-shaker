use std::{cell::UnsafeCell, mem, rc::Rc};

use line_index::LineIndex;
use oxc::{
  allocator::FromIn,
  ast::ast::{ImportDeclaration, Program, Statement},
  parser::Parser,
  semantic::{Semantic, SemanticBuilder, SymbolId},
  span::{Atom, SourceType},
};
use oxc_index::{IndexVec, define_index_type};
use rustc_hash::FxHashMap;

use crate::{
  analyzer::Analyzer,
  consumable::ConsumableTrait,
  entity::Entity,
  scope::{
    CfScopeId, CfScopeKind, VariableScopeId, call_scope::CallScope, cf_scope::CfScope,
    variable_scope::VariableScope,
  },
  utils::{CalleeInfo, CalleeNode, dep_id::DepId},
};

#[derive(Clone)]
pub struct ModuleInfo<'a> {
  pub path: Atom<'a>,
  pub line_index: LineIndex,
  pub program: &'a UnsafeCell<Program<'a>>,
  pub semantic: Rc<Semantic<'a>>,
  pub call_id: DepId,

  pub named_exports: FxHashMap<Atom<'a>, (VariableScopeId, SymbolId)>,
  pub default_export: Option<Entity<'a>>,

  pub blocked_imports: Vec<(ModuleId, VariableScopeId, &'a ImportDeclaration<'a>)>,
}

define_index_type! {
  pub struct ModuleId = u32;
}

#[derive(Default)]
pub struct Modules<'a> {
  pub modules: IndexVec<ModuleId, ModuleInfo<'a>>,
  paths: FxHashMap<String, ModuleId>,
}

impl<'a> Analyzer<'a> {
  pub fn module_info(&self) -> &ModuleInfo<'a> {
    &self.modules.modules[self.current_module()]
  }

  pub fn module_info_mut(&mut self) -> &mut ModuleInfo<'a> {
    let module_id = self.current_module();
    &mut self.modules.modules[module_id]
  }

  pub fn semantic<'b>(&'b self) -> &'b Semantic<'a> {
    &self.module_info().semantic
  }

  pub fn line_index(&self) -> &LineIndex {
    &self.module_info().line_index
  }

  pub fn resolve_and_import_module(&mut self, specifier: &str) -> Option<ModuleId> {
    let importer = &self.module_info().path;
    let path = self.vfs.resolve_module(importer, specifier)?;
    Some(self.import_module(path))
  }

  pub fn import_module(&mut self, path: String) -> ModuleId {
    let path = self.vfs.normalize_path(path);

    if let Some(module_id) = self.modules.paths.get(path.as_str()) {
      return *module_id;
    }

    let source_text = self.allocator.alloc_str(&self.vfs.read_file(path.as_str()));
    let line_index = LineIndex::new(source_text);
    let parser = Parser::new(
      self.allocator,
      source_text,
      SourceType::mjs().with_jsx(self.config.jsx.is_enabled()),
    );
    let parsed = parser.parse();
    let program = self.allocator.alloc(UnsafeCell::new(parsed.program));
    for error in parsed.errors {
      self.add_diagnostic(format!("[{}] {}", path, error));
    }
    let semantic = SemanticBuilder::new().build(unsafe { &*program.get() }).semantic;
    let semantic = Rc::new(semantic);
    let module_id = self.modules.modules.push(ModuleInfo {
      path: Atom::from_in(path.clone(), self.allocator),
      line_index,
      program,
      semantic,
      call_id: DepId::from_counter(),

      named_exports: Default::default(),
      default_export: Default::default(),

      blocked_imports: Vec::new(),
    });
    self.modules.paths.insert(path.clone(), module_id);

    self.exec_module(module_id);

    module_id
  }

  fn exec_module(&mut self, module_id: ModuleId) {
    let ModuleInfo { call_id, program, .. } = self.modules.modules[module_id].clone();
    self.module_stack.push(module_id);
    let old_variable_scope_stack = self.replace_variable_scope_stack(vec![]);
    let root_variable_scope =
      self.scoping.variable.push(VariableScope::new_with_this(self.factory.unknown()));
    self.scoping.call.push(CallScope::new_in(
      call_id,
      CalleeInfo {
        module_id,
        node: CalleeNode::Module,
        instance_id: self.factory.alloc_instance_id(),
        #[cfg(feature = "flame")]
        debug_name: "<Module>",
      },
      vec![],
      0,
      root_variable_scope,
      true,
      false,
      self.allocator,
    ));
    let old_cf_scope_stack = self.scoping.cf.replace_stack(vec![CfScopeId::from(0)]);
    self.scoping.cf.push(CfScope::new(CfScopeKind::Module, self.factory.vec(), Some(false)));

    let program = unsafe { &*program.get() };
    for node in &program.body {
      self.declare_statement(node);
    }
    for node in &program.body {
      if let Statement::ImportDeclaration(node) = node {
        self.init_import_declaration(node);
      }
    }
    for node in &program.body {
      self.init_statement(node);
    }

    self.scoping.cf.replace_stack(old_cf_scope_stack);
    self.scoping.call.pop();
    self.replace_variable_scope_stack(old_variable_scope_stack);
    self.module_stack.pop();

    for (module, scope, node) in mem::take(&mut self.modules.modules[module_id].blocked_imports) {
      self.module_stack.push(module);
      self.scoping.variable.stack.push(scope);
      self.init_import_declaration(node);
      self.scoping.variable.stack.pop();
      self.module_stack.pop();
    }
  }

  pub fn consume_exports(&mut self, module_id: ModuleId) {
    let ModuleInfo { call_id, named_exports, default_export, .. } =
      self.modules.modules[module_id].clone();
    self.refer_dep(call_id);
    for (scope, symbol) in named_exports.into_values() {
      self.consume_on_scope(scope, symbol);
    }
    if let Some(entity) = default_export {
      self.consume(entity);
    }
  }
}

impl ConsumableTrait<'_> for ModuleId {
  fn consume(&self, analyzer: &mut Analyzer) {
    analyzer.consume_exports(*self);
  }
}
