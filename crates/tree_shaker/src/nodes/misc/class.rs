use std::rc::Rc;

use crate::{
  analyzer::Analyzer,
  ast::{AstKind2, DeclarationKind},
  consumable::Consumable,
  entity::{Entity, EntityTrait, ObjectPrototype},
  scope::VariableScopeId,
  transformer::Transformer,
  utils::{CalleeInfo, CalleeNode},
};

use oxc::{
  allocator,
  ast::{
    ast::{
      Class, ClassBody, ClassElement, ClassType, MethodDefinition, MethodDefinitionKind,
      PropertyDefinitionType, PropertyKind, StaticBlock,
    },
    NONE,
  },
  span::GetSpan,
};

#[derive(Default)]
struct Data<'a> {
  pub constructor: Option<&'a MethodDefinition<'a>>,
  pub keys: Vec<Option<Entity<'a>>>,
  pub super_class: Option<Entity<'a>>,
}

impl<'a> Analyzer<'a> {
  pub fn exec_class(&mut self, node: &'a Class<'a>) -> Entity<'a> {
    let data = self.load_data::<Data>(AstKind2::Class(node));
    let class = self.new_function(CalleeNode::ClassConstructor(node));

    // 1. Execute super class
    data.super_class = node.super_class.as_ref().map(|node| self.exec_expression(node));
    if let Some(super_class) = &data.super_class {
      // Because we can't re-define the "prototype" property, this should be side-effect free
      if let Some((prototype_dep, super_statics, super_prototype)) =
        super_class.get_constructor_prototype(self, self.factory.empty_consumable)
      {
        class.statics.prototype.set(super_statics);
        class.prototype.prototype.set(super_prototype);
        class.prototype.unknown_mutate(self, prototype_dep);
      } else {
        let dep = self.factory.consumable(*super_class);
        class.statics.prototype.set(ObjectPrototype::Unknown(dep));
        class.prototype.prototype.set(ObjectPrototype::Unknown(dep));
      }
    } else {
      class.prototype.prototype.set(ObjectPrototype::ImplicitOrNull);
    };

    self.push_variable_scope();
    self.variable_scope_mut().super_class =
      Some(data.super_class.unwrap_or(self.factory.undefined));

    // 2. Execute keys and find constructor
    for element in &node.body.body {
      let key = element.property_key().map(|key| self.exec_property_key(key));
      data.keys.push(key);

      if let ClassElement::MethodDefinition(method) = element {
        if method.kind.is_constructor() {
          if data.constructor.is_some() {
            self.throw_builtin_error("A class may only have one constructor");
          }
          data.constructor = Some(method);
        }
      }
    }

    // 3. Register methods
    for (key, element) in data.keys.iter().zip(node.body.body.iter()) {
      if let ClassElement::MethodDefinition(node) = element {
        let kind = match node.kind {
          MethodDefinitionKind::Constructor => continue,
          MethodDefinitionKind::Method => PropertyKind::Init,
          MethodDefinitionKind::Get => PropertyKind::Get,
          MethodDefinitionKind::Set => PropertyKind::Set,
        };
        let value = self.exec_function(&node.value);
        if node.r#static {
          class.statics.init_property(self, kind, key.unwrap(), value, true);
        } else {
          class.prototype.init_property(self, kind, key.unwrap(), value, true);
        }
      }
    }

    // 4. Execute static blocks
    let variable_scope_stack = self.scoping.variable.stack.clone();
    self.push_call_scope(
      self.new_callee_info(CalleeNode::ClassStatics(node)),
      self.factory.empty_consumable,
      variable_scope_stack,
      false,
      false,
      false,
    );

    if let Some(id) = &node.id {
      self.declare_binding_identifier(id, false, DeclarationKind::NamedFunctionInBody);
      self.init_binding_identifier(id, Some(class));
    }

    for (index, element) in node.body.body.iter().enumerate() {
      match element {
        ClassElement::StaticBlock(node) => self.exec_static_block(node),
        ClassElement::MethodDefinition(_node) => {}
        ClassElement::PropertyDefinition(node) if node.r#static => {
          if let Some(value) = &node.value {
            let key = data.keys[index].unwrap();
            let value = self.factory.computed(
              self.exec_expression(value),
              self.consumable(AstKind2::PropertyDefinition(node)),
            );
            class.statics.init_property(self, PropertyKind::Init, key, value, true);
          }
        }
        _ => {}
      }
    }

    self.pop_call_scope();
    self.pop_variable_scope();

    class
  }

  pub fn declare_class(&mut self, node: &'a Class<'a>, exporting: bool) {
    self.declare_binding_identifier(node.id.as_ref().unwrap(), exporting, DeclarationKind::Class);
  }

  pub fn init_class(&mut self, node: &'a Class<'a>) -> Entity<'a> {
    let value = self.exec_class(node);

    self.init_binding_identifier(node.id.as_ref().unwrap(), Some(value));

    value
  }

  pub fn call_class_constructor(
    &mut self,
    callee: CalleeInfo<'a>,
    call_dep: Consumable<'a>,
    node: &'a Class<'a>,
    variable_scopes: Rc<Vec<VariableScopeId>>,
    this: Entity<'a>,
    args: Entity<'a>,
    consume: bool,
  ) -> Entity<'a> {
    let data = self.load_data::<Data>(AstKind2::Class(node));

    self.push_call_scope(callee, call_dep, variable_scopes.as_ref().clone(), false, false, consume);
    let super_class = data.super_class.unwrap_or(self.factory.undefined);
    let variable_scope = self.variable_scope_mut();
    variable_scope.this = Some(this);
    variable_scope.arguments = Some((args, vec![ /* later filled by formal parameters */ ]));
    variable_scope.super_class = Some(super_class);

    // 1. Init properties
    for (key, element) in data.keys.iter().zip(node.body.body.iter()) {
      if let ClassElement::PropertyDefinition(node) = element {
        if !node.r#static {
          if let Some(value) = &node.value {
            let value = self.exec_expression(value);
            this.set_property(
              self,
              self.factory.consumable(AstKind2::PropertyDefinition(node)),
              key.unwrap(),
              value,
            );
          }
        }
      }
    }

    // 2. Call constructor
    if let Some(constructor) = data.constructor {
      let function = constructor.value.as_ref();
      let dep = self.factory.consumable(AstKind2::Function(function));
      self.cf_scope_mut().push_dep(dep);
      self.exec_formal_parameters(&function.params, args, DeclarationKind::FunctionParameter);
      self.exec_function_body(function.body.as_ref().unwrap());
      if consume {
        self.consume_return_values();
      }
    }

    self.pop_call_scope()
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_class(
    &self,
    node: &'a Class<'a>,
    need_val: bool,
  ) -> Option<allocator::Box<'a, Class<'a>>> {
    let Class { r#type, span, id, super_class, body, .. } = node;

    let transformed_id = id.as_ref().and_then(|node| self.transform_binding_identifier(node));

    if need_val || transformed_id.is_some() {
      let id = if self.config.preserve_function_name {
        self.clone_node(id)
      } else if node.r#type == ClassType::ClassDeclaration && id.is_some() {
        // `id` cannot be omitted for class declaration
        // However, we still check `id.is_some()` to handle `export default class {}`
        Some(
          transformed_id
            .unwrap_or_else(|| self.build_unused_binding_identifier(id.as_ref().unwrap().span)),
        )
      } else {
        transformed_id
      };

      let super_class = super_class.as_ref().and_then(|node| self.transform_expression(node, true));

      let body = {
        let ClassBody { span, body } = body.as_ref();

        let mut transformed_body = self.ast_builder.vec();

        for element in body {
          if let Some(element) = match element {
            ClassElement::StaticBlock(node) => {
              self.transform_static_block(node).map(ClassElement::StaticBlock)
            }
            ClassElement::MethodDefinition(node) => self.transform_method_definition(node),
            ClassElement::PropertyDefinition(node) => self.transform_property_definition(node),
            ClassElement::AccessorProperty(_node) => unreachable!(),
            ClassElement::TSIndexSignature(_node) => unreachable!(),
          } {
            transformed_body.push(element);
          } else if let Some(key) =
            element.property_key().and_then(|key| self.transform_property_key(key, false))
          {
            transformed_body.push(self.ast_builder.class_element_property_definition(
              element.span(),
              PropertyDefinitionType::PropertyDefinition,
              self.ast_builder.vec(),
              key,
              None,
              false,
              false,
              false,
              false,
              false,
              false,
              false,
              NONE,
              None,
            ));
          }
        }

        self.ast_builder.class_body(*span, transformed_body)
      };

      Some(self.ast_builder.alloc_class(
        *span,
        *r#type,
        self.ast_builder.vec(),
        id,
        NONE,
        super_class,
        NONE,
        None,
        body,
        false,
        false,
      ))
    } else {
      // Side-effect only

      let mut statements = self.ast_builder.vec();

      if let Some(super_class) = super_class {
        let span = super_class.span();
        if let Some(super_class) = self.transform_expression(super_class, false) {
          statements.push(self.ast_builder.statement_expression(span, super_class));
        }
      }

      for element in &body.body {
        if let Some(key) = element.property_key() {
          if key.is_expression() {
            if let Some(element) = self.transform_expression(key.to_expression(), false) {
              statements.push(self.ast_builder.statement_expression(element.span(), element));
            }
          }
        }
      }

      for element in &body.body {
        match element {
          ClassElement::StaticBlock(node) => {
            if let Some(node) = self.transform_static_block(node) {
              let StaticBlock { span, body, .. } = node.unbox();
              statements.push(self.ast_builder.statement_block(span, body));
            }
          }
          ClassElement::PropertyDefinition(node) if node.r#static => {
            if let Some(value) = &node.value {
              let span = value.span();
              if let Some(value) = self.transform_expression(value, false) {
                statements.push(self.ast_builder.statement_expression(span, value));
              }
            }
          }
          _ => {}
        }
      }

      if statements.is_empty() {
        None
      } else {
        Some(
          self.ast_builder.alloc_class(
            *span,
            *r#type,
            self.ast_builder.vec(),
            (node.r#type == ClassType::ClassDeclaration)
              .then(|| self.build_unused_binding_identifier(id.as_ref().unwrap().span)),
            NONE,
            None,
            NONE,
            None,
            self.ast_builder.class_body(
              body.span(),
              self
                .ast_builder
                .vec1(self.ast_builder.class_element_static_block(body.span(), statements)),
            ),
            false,
            false,
          ),
        )
      }
    }
  }
}
