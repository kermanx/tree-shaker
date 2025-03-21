use rustc_hash::FxHashMap;
use std::mem;

use crate::{analyzer::Analyzer, dep::DepId, entity::Entity};

pub struct LoopData<'a> {
  call_id: DepId,
  tests: Vec<Entity<'a>>,
}

pub type LoopDataMap<'a> = FxHashMap<DepId, Vec<LoopData<'a>>>;

impl Analyzer<'_> {
  pub fn post_analyze_handle_loops(&mut self) -> bool {
    let mut remained = LoopDataMap::default();
    let mut dirty = false;
    for (loop_dep, data) in mem::take(&mut self.loop_data) {
      if self.is_referred(loop_dep) {
        let mut remained_data = vec![];
        for data in data {
          if self.is_referred(data.call_id) {
            self.consume(data.tests);
            dirty = true;
          } else {
            remained_data.push(data);
          }
        }
        if !remained_data.is_empty() {
          remained.insert(loop_dep, remained_data);
        }
      } else {
        remained.insert(loop_dep, data);
      }
    }
    dirty
  }
}
