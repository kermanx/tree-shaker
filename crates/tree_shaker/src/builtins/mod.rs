mod constants;
mod globals;
mod import_meta;
mod known_modules;
mod prototypes;
mod react;
mod utils;

use crate::{
  TreeShakeConfig,
  entity::{Entity, EntityFactory},
};
use known_modules::KnownModule;
pub use prototypes::BuiltinPrototype;
use prototypes::BuiltinPrototypes;
use react::AnalyzerDataForReact;
use rustc_hash::FxHashMap;

pub struct Builtins<'a> {
  pub config: &'a TreeShakeConfig,
  pub factory: &'a EntityFactory<'a>,

  pub prototypes: &'a BuiltinPrototypes<'a>,
  pub globals: FxHashMap<&'static str, Entity<'a>>,
  pub import_meta: Entity<'a>,
  pub known_modules: FxHashMap<&'static str, KnownModule<'a>>,

  pub react_data: AnalyzerDataForReact<'a>,
}

impl<'a> Builtins<'a> {
  pub fn new(config: &'a TreeShakeConfig, factory: &'a EntityFactory<'a>) -> Self {
    let prototypes = Self::create_builtin_prototypes(factory);
    let mut builtins = Self {
      config,
      factory,

      prototypes,
      import_meta: Self::create_import_meta(factory, prototypes),
      globals: Default::default(),       // Initialize later
      known_modules: Default::default(), // Initialize later

      react_data: Default::default(),
    };
    builtins.init_globals();
    builtins.init_known_modules();
    builtins
  }
}
