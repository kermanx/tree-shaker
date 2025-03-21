mod analyzer;
mod builtins;
mod config;
mod consumable;
mod entity;
mod folding;
mod mangling;
mod module;
mod nodes;
mod scope;
mod transformer;
mod utils;
pub mod vfs;

use std::{cell::RefCell, collections::BTreeSet, mem, rc::Rc};

use analyzer::Analyzer;
use mangling::ManglerTransformer;
use module::ModuleInfo;
use oxc::{
  allocator::Allocator,
  codegen::{CodeGenerator, CodegenOptions, CodegenReturn},
  minifier::{Minifier, MinifierOptions},
  parser::Parser,
  span::SourceType,
};
use oxc_ast_visit::VisitMut;
use rustc_hash::FxHashMap;
use transformer::Transformer;
use utils::{
  Diagnostics, ast,
  dep_id::{self as dep},
};

pub use config::{TreeShakeConfig, TreeShakeJsxPreset};
use vfs::Vfs;

pub struct TreeShakeOptions<F: Vfs> {
  pub vfs: F,
  pub config: TreeShakeConfig,
  pub minify_options: Option<MinifierOptions>,
  pub codegen_options: CodegenOptions,
}

pub struct TreeShakeReturn {
  pub codegen_return: FxHashMap<String, CodegenReturn>,
  pub diagnostics: Diagnostics,
}

pub fn tree_shake<F: Vfs + 'static>(
  options: TreeShakeOptions<F>,
  entry: String,
) -> TreeShakeReturn {
  let TreeShakeOptions { vfs, config, minify_options, codegen_options } = options;

  if config.enabled {
    let allocator = Allocator::default();
    let config = allocator.alloc(config);

    // Step 1: Analyze
    let mut analyzer = Analyzer::new_in(Box::new(vfs), config, &allocator);
    analyzer.import_module(entry);
    analyzer.finalize();
    let Analyzer {
      modules,
      diagnostics,
      folder,
      mangler,
      data,
      referred_deps,
      conditional_data,
      ..
    } = unsafe { &mut *(&mut analyzer as *mut _) };
    let mangler = Rc::new(RefCell::new(mangler));
    let mut codegen_return = FxHashMap::default();
    for module_info in mem::take(&mut modules.modules) {
      let ModuleInfo { path, program, semantic, .. } = module_info;

      // Step 2: Transform
      let transformer = Transformer::new(
        config,
        &allocator,
        data,
        referred_deps,
        conditional_data,
        folder,
        mangler.clone(),
        semantic,
      );
      let program = if config.mangling == Some(true) {
        // Mangling only
        let program = unsafe { &mut *program.get() };
        ManglerTransformer(transformer).visit_program(program);
        program
      } else {
        let program = unsafe { &*program.get() };
        allocator.alloc(transformer.transform_program(program))
      };

      // Step 3: Minify
      let minifier_return = minify_options.map(|options| {
        let minifier = Minifier::new(options);
        minifier.build(&allocator, program)
      });

      // Step 4: Generate output
      let codegen = CodeGenerator::new()
        .with_options(codegen_options.clone())
        .with_scoping(minifier_return.and_then(|r| r.scoping));
      codegen_return.insert(path.to_string(), codegen.build(program));
    }
    TreeShakeReturn { codegen_return, diagnostics: mem::take(diagnostics) }
  } else {
    let allocator = Allocator::default();
    let config = allocator.alloc(config);

    let source_text = vfs.read_file(&entry);
    let parser =
      Parser::new(&allocator, &source_text, SourceType::mjs().with_jsx(config.jsx.is_enabled()));
    let parsed = parser.parse();
    let mut program = parsed.program;
    let minifier_return = minify_options.map(|options| {
      let minifier = Minifier::new(options);
      minifier.build(&allocator, &mut program)
    });
    let codegen = CodeGenerator::new()
      .with_options(codegen_options.clone())
      .with_scoping(minifier_return.and_then(|r| r.scoping));
    let mut codegen_return = FxHashMap::default();
    codegen_return.insert(entry, codegen.build(&program));
    let mut diagnostics = BTreeSet::<String>::default();
    for error in parsed.errors {
      diagnostics.insert(error.to_string());
    }
    TreeShakeReturn { codegen_return, diagnostics }
  }
}
