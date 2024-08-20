use crate::{entity::Entity, symbol::SymbolSource, TreeShaker};
use oxc::{
  ast::ast::{BindingPattern, BindingPatternKind},
  semantic::SymbolId,
};
use rustc_hash::FxHashSet;

#[derive(Debug, Default, Clone)]
pub struct Data {
  referred_symbols: FxHashSet<SymbolId>,
}

impl<'a> TreeShaker<'a> {
  pub(crate) fn exec_binding_pattern(
    &mut self,
    node: &'a BindingPattern<'a>,
    symbol_source: SymbolSource<'a>,
  ) {
    let data = self.load_data::<Data>(node);

    match &node.kind {
      BindingPatternKind::BindingIdentifier(node) => {
        let symbol = node.symbol_id.get().unwrap();
        self.declare_symbol(symbol_source, symbol);
      }
      BindingPatternKind::ObjectPattern(node) => {
        for property in &node.properties {
          self.exec_property_key(&property.key);
          self.exec_binding_pattern(&property.value, symbol_source.clone());
        }
      }
      BindingPatternKind::ArrayPattern(node) => {
        for element in &node.elements {
          if let Some(element) = element {
            self.exec_binding_pattern(element, symbol_source.clone());
          }
        }
      }
      BindingPatternKind::AssignmentPattern(node) => {
        self.exec_binding_pattern(&node.left, symbol_source);
      }
    }
    todo!()
  }

  pub(crate) fn refer_binding_pattern(
    &mut self,
    node: &'a BindingPattern,
    symbol: SymbolId,
    init_val: Entity,
  ) -> Entity {
    let data = self.load_data::<Data>(node);
    data.referred_symbols.insert(symbol);

    match &node.kind {
      BindingPatternKind::BindingIdentifier(node) => {
        assert!(node.symbol_id.get().unwrap() == symbol);
        init_val
      }
      BindingPatternKind::ObjectPattern(node) => {
        for property in &node.properties {
          if self.is_in_binding_pattern(&property.value, symbol) {
            let key = self.exec_property_key(&property.key);
            let value = init_val.get_property(&key).as_ref().clone();
            return self.refer_binding_pattern(&property.value, symbol, value);
          }
        }
        todo!("rest property")
      }
      BindingPatternKind::ArrayPattern(node) => {
        for (index, element) in node.elements.iter().enumerate() {
          if let Some(element) = element {
            if self.is_in_binding_pattern(&element, symbol) {
              let key = Entity::NumberLiteral(index as f64);
              let value = init_val.get_property(&key).as_ref().clone();
              return self.refer_binding_pattern(&element, symbol, value);
            }
          }
        }
        todo!("rest property")
      }
      BindingPatternKind::AssignmentPattern(node) => {
        let value = self.refer_binding_pattern(&node.left, symbol, init_val);
        if value.is_null_or_undefined() {
          // FIXME:
          self.exec_expression(&node.right);
        }
        value
      }
      _ => todo!(),
    }
  }

  fn is_in_binding_pattern(&self, node: &'a BindingPattern, symbol_id: SymbolId) -> bool {
    match &node.kind {
      BindingPatternKind::BindingIdentifier(node) => node.symbol_id.get().unwrap() == symbol_id,
      BindingPatternKind::ObjectPattern(node) => {
        for property in &node.properties {
          if self.is_in_binding_pattern(&property.value, symbol_id) {
            return true;
          }
        }
        false
      }
      BindingPatternKind::ArrayPattern(node) => {
        for element in &node.elements {
          if let Some(element) = element {
            if self.is_in_binding_pattern(element, symbol_id) {
              return true;
            }
          }
        }
        false
      }
      BindingPatternKind::AssignmentPattern(node) => {
        self.is_in_binding_pattern(&node.left, symbol_id)
      }
    }
  }
}
