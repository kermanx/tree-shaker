use std::rc::Rc;

use crate::{entity::Entity, symbol::SymbolSource, Analyzer};
use oxc::{
  ast::ast::{
    BindingPattern, BindingPatternKind, BindingRestElement, FormalParameter, VariableDeclarator,
  },
  semantic::SymbolId,
};

#[derive(Debug, Clone, Copy)]
pub(crate) enum BindingPatternSource<'a> {
  VariableDeclarator(&'a VariableDeclarator<'a>),
  FormalParameter(&'a FormalParameter<'a>),
  BindingRestElement(&'a BindingRestElement<'a>),
}

impl<'a> BindingPatternSource<'a> {
  pub(self) fn to_symble_source(&self, symbol: SymbolId) -> SymbolSource<'a> {
    match self {
      BindingPatternSource::VariableDeclarator(node) => {
        SymbolSource::VariableDeclarator(node, symbol)
      }
      BindingPatternSource::FormalParameter(node) => SymbolSource::FormalParameter(node, symbol),
      BindingPatternSource::BindingRestElement(node) => {
        SymbolSource::BindingRestElement(node, symbol)
      }
    }
  }
}

#[derive(Debug, Default, Clone)]
pub struct Data {
  init_val: Entity,
  referred: bool,
}

impl<'a> Analyzer<'a> {
  pub(crate) fn exec_binding_pattern(
    &mut self,
    node: &'a BindingPattern<'a>,
    source: BindingPatternSource<'a>,
    init_val: Entity,
  ) -> bool {
    let mut effect = false;
    match &node.kind {
      BindingPatternKind::BindingIdentifier(node) => {
        let symbol = node.symbol_id.get().unwrap();
        self.declare_symbol(source.to_symble_source(symbol), symbol);
      }
      BindingPatternKind::ObjectPattern(node) => {
        for property in &node.properties {
          let (key_effect, key_val) = self.exec_property_key(&property.key);
          effect |= key_effect;
          effect |= self.exec_binding_pattern(
            &property.value,
            source,
            (*init_val.get_property(&key_val)).clone(),
          );
        }
        // TODO: rest
      }
      BindingPatternKind::ArrayPattern(node) => {
        for (index, element) in node.elements.iter().enumerate() {
          if let Some(element) = element {
            let key_val = Entity::StringLiteral(index.to_string());
            effect |= self.exec_binding_pattern(
              element,
              source,
              (*init_val.get_property(&key_val)).clone(),
            );
          }
        }
        // TODO: rest
      }
      BindingPatternKind::AssignmentPattern(node) => {
        let is_nullable = init_val.is_nullable();
        let binding_val = match is_nullable {
          Some(true) => self.calc_expression(&node.right),
          Some(false) => init_val.clone(),
          None => Entity::Union(vec![
            Rc::new(self.calc_expression(&node.right)),
            Rc::new(init_val.clone()),
          ])
          .simplify(),
        };
        effect |= self.exec_binding_pattern(&node.left, source, binding_val);
        effect |= match is_nullable {
          Some(true) => self.exec_expression(&node.right).0,
          Some(false) => false,
          None => {
            let backup = self.start_indeterminate();
            let (right_effect, _) = self.exec_expression(&node.right);
            self.end_indeterminate(backup);
            right_effect
          }
        };
      }
    }

    self.set_data(node, Data { init_val, referred: false });

    effect
  }

  pub(crate) fn calc_binding_pattern(
    &self,
    node: &'a BindingPattern<'a>,
    symbol: SymbolId,
  ) -> Option<Entity> {
    let data = self.get_data::<Data>(node);

    match &node.kind {
      BindingPatternKind::BindingIdentifier(node) => {
        (node.symbol_id.get().unwrap() == symbol).then(|| data.init_val.clone())
      }
      BindingPatternKind::ObjectPattern(node) => {
        for property in &node.properties {
          if let Some(val) = self.calc_binding_pattern(&property.value, symbol) {
            return Some(val);
          }
        }
        node.rest.as_ref().and_then(|rest| self.calc_binding_rest_element(rest, symbol))
      }
      BindingPatternKind::ArrayPattern(node) => {
        for element in &node.elements {
          if let Some(element) = element {
            if let Some(val) = self.calc_binding_pattern(&element, symbol) {
              return Some(val);
            }
          }
        }
        node.rest.as_ref().and_then(|rest| self.calc_binding_rest_element(rest, symbol))
      }
      BindingPatternKind::AssignmentPattern(node) => self.calc_binding_pattern(&node.left, symbol),
    }
  }

  pub(crate) fn refer_binding_pattern(&mut self, node: &'a BindingPattern, symbol: SymbolId) {
    let data = self.load_data::<Data>(node);

    match &node.kind {
      BindingPatternKind::BindingIdentifier(node) => {
        data.referred |= node.symbol_id.get().unwrap() == symbol;
      }
      BindingPatternKind::ObjectPattern(node) => {
        for property in &node.properties {
          self.refer_binding_pattern(&property.value, symbol);
        }
        node.rest.as_ref().map(|rest| self.refer_binding_rest_element(rest, symbol));
      }
      BindingPatternKind::ArrayPattern(node) => {
        for (index, element) in node.elements.iter().enumerate() {
          if let Some(element) = element {
            self.refer_binding_pattern(&element, symbol);
          }
        }
        node.rest.as_ref().map(|rest| self.refer_binding_rest_element(rest, symbol));
      }
      BindingPatternKind::AssignmentPattern(node) => {
        self.refer_binding_pattern(&node.left, symbol);
      }
    }
  }

  // fn is_in_binding_pattern(&self, node: &'a BindingPattern, symbol_id: SymbolId) -> bool {
  //   match &node.kind {
  //     BindingPatternKind::BindingIdentifier(node) => node.symbol_id.get().unwrap() == symbol_id,
  //     BindingPatternKind::ObjectPattern(node) => {
  //       for property in &node.properties {
  //         if self.is_in_binding_pattern(&property.value, symbol_id) {
  //           return true;
  //         }
  //       }
  //       false
  //     }
  //     BindingPatternKind::ArrayPattern(node) => {
  //       for element in &node.elements {
  //         if let Some(element) = element {
  //           if self.is_in_binding_pattern(element, symbol_id) {
  //             return true;
  //           }
  //         }
  //       }
  //       false
  //     }
  //     BindingPatternKind::AssignmentPattern(node) => {
  //       self.is_in_binding_pattern(&node.left, symbol_id)
  //     }
  //   }
  // }
}
