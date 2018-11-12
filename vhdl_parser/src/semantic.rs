// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this file,
// You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) 2018, Olof Kraigher olof.kraigher@gmail.com

use ast::*;
use message::{error, MessageHandler};
use source::SrcPos;
use symbol_table::Symbol;

extern crate fnv;
use self::fnv::FnvHashMap;
use std::collections::hash_map::Entry;

impl Declaration {
    fn ident<'a>(&'a self) -> Option<&'a Ident> {
        match self {
            // @TODO Ignored for now
            Declaration::Alias(AliasDeclaration { .. }) => None,
            Declaration::Object(ObjectDeclaration { ref ident, .. }) => Some(ident),
            Declaration::File(FileDeclaration { ref ident, .. }) => Some(ident),
            Declaration::Component(ComponentDeclaration { ref ident, .. }) => Some(ident),
            // @TODO Ignored for now
            Declaration::Attribute(..) => None,
            // @TODO Ignored for now
            Declaration::SubprogramBody(..) => None,
            // @TODO Ignored for now
            Declaration::SubprogramDeclaration(..) => None,
            // @TODO Ignored for now
            Declaration::Use(..) => None,
            // @TODO Ignored for now
            Declaration::Package(..) => None,
            Declaration::Configuration(..) => None,
            Declaration::Type(TypeDeclaration {
                def: TypeDefinition::ProtectedBody(..),
                ..
            }) => None,
            Declaration::Type(TypeDeclaration {
                def: TypeDefinition::Incomplete,
                ..
            }) => None,
            Declaration::Type(TypeDeclaration { ref ident, .. }) => Some(ident),
        }
    }
}

impl InterfaceDeclaration {
    fn ident<'a>(&'a self) -> Option<&'a Ident> {
        match self {
            InterfaceDeclaration::File(InterfaceFileDeclaration { ref ident, .. }) => Some(ident),
            InterfaceDeclaration::Object(InterfaceObjectDeclaration { ref ident, .. }) => {
                Some(ident)
            }
            InterfaceDeclaration::Type(ref ident) => Some(ident),
            // @TODO ignore for now
            InterfaceDeclaration::Subprogram(..) => None,
        }
    }
}
fn check_unique<'a>(
    idents: &mut FnvHashMap<&'a Symbol, &'a SrcPos>,
    ident: &'a Ident,
    messages: &mut MessageHandler,
) {
    match idents.entry(&ident.item) {
        Entry::Occupied(entry) => {
            let msg = error(
                ident,
                &format!("Duplicate declaration of '{}'", ident.item.name()),
            ).related(entry.get(), "Previously defined here");
            messages.push(msg)
        }
        Entry::Vacant(entry) => {
            entry.insert(&ident.pos);
        }
    }
}

/// Check that no homographs are defined in the element declarations
fn check_element_declaration_unique_ident(
    declarations: &[ElementDeclaration],
    messages: &mut MessageHandler,
) {
    let mut idents = FnvHashMap::default();
    for decl in declarations.iter() {
        check_unique(&mut idents, &decl.ident, messages);
    }
}

/// Check that no homographs are defined in the interface list
fn check_interface_list_unique_ident(
    declarations: &[InterfaceDeclaration],
    messages: &mut MessageHandler,
) {
    let mut idents = FnvHashMap::default();
    for decl in declarations.iter() {
        if let Some(ident) = decl.ident() {
            check_unique(&mut idents, ident, messages);
        }
    }
}

impl SubprogramDeclaration {
    fn interface_list<'a>(&'a self) -> &[InterfaceDeclaration] {
        match self {
            SubprogramDeclaration::Function(fun) => &fun.parameter_list,
            SubprogramDeclaration::Procedure(proc) => &proc.parameter_list,
        }
    }
}

/// Check that no homographs are defined in the declarative region
fn check_declarative_part_unique_ident(
    declarations: &[Declaration],
    messages: &mut MessageHandler,
) {
    let mut idents = FnvHashMap::default();
    for decl in declarations.iter() {
        if let Some(ident) = decl.ident() {
            check_unique(&mut idents, ident, messages);
        }

        match decl {
            Declaration::Component(ref component) => {
                check_interface_list_unique_ident(&component.generic_list, messages);
                check_interface_list_unique_ident(&component.port_list, messages);
            }
            Declaration::SubprogramBody(ref body) => {
                check_interface_list_unique_ident(body.specification.interface_list(), messages);
                check_declarative_part_unique_ident(&body.declarations, messages);
            }
            Declaration::SubprogramDeclaration(decl) => {
                check_interface_list_unique_ident(decl.interface_list(), messages);
            }
            Declaration::Type(type_decl) => match type_decl.def {
                TypeDefinition::ProtectedBody(ref body) => {
                    check_declarative_part_unique_ident(&body.decl, messages);
                }
                TypeDefinition::Protected(ref prot_decl) => {
                    for item in prot_decl.items.iter() {
                        match item {
                            ProtectedTypeDeclarativeItem::Subprogram(subprogram) => {
                                check_interface_list_unique_ident(
                                    subprogram.interface_list(),
                                    messages,
                                );
                            }
                        }
                    }
                }
                TypeDefinition::Record(ref decls) => {
                    check_element_declaration_unique_ident(decls, messages);
                }
                _ => {}
            },
            _ => {}
        }
    }
}

fn check_package_declaration(package: &PackageDeclaration, messages: &mut MessageHandler) {
    check_declarative_part_unique_ident(&package.decl, messages);
}

fn check_architecture_body(architecture: &ArchitectureBody, messages: &mut MessageHandler) {
    check_declarative_part_unique_ident(&architecture.decl, messages);
    // @TODO declarative parts in concurrent statements
}

fn check_package_body(package: &PackageBody, messages: &mut MessageHandler) {
    check_declarative_part_unique_ident(&package.decl, messages);
}

fn check_entity_declaration(entity: &EntityDeclaration, messages: &mut MessageHandler) {
    if let Some(ref list) = entity.generic_clause {
        check_interface_list_unique_ident(list, messages);
    }
    if let Some(ref list) = entity.port_clause {
        check_interface_list_unique_ident(list, messages);
    }
    check_declarative_part_unique_ident(&entity.decl, messages);
    // @TODO declarative parts in concurrent statements
}

pub fn check_design_unit(design_unit: &DesignUnit, messages: &mut MessageHandler) {
    match &design_unit.library_unit {
        LibraryUnit::PackageDeclaration(package) => check_package_declaration(package, messages),
        LibraryUnit::Architecture(architecture) => check_architecture_body(architecture, messages),
        LibraryUnit::PackageBody(package) => check_package_body(package, messages),
        LibraryUnit::EntityDeclaration(entity) => check_entity_declaration(entity, messages),
        // @TODO others
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use message::Message;
    use test_util::{check_no_messages, Code};

    fn expected_messages(code: &Code, num: usize) -> Vec<Message> {
        let mut messages = Vec::new();
        for i in 0..num {
            let chr = (b'a' + (i as u8)) as char;
            let name = format!("{}1", chr);
            messages.push(
                error(
                    code.s(&name, 2),
                    &format!("Duplicate declaration of '{}'", &name),
                ).related(code.s1(&name), "Previously defined here"),
            )
        }
        messages
    }

    #[test]
    fn allows_unique_names() {
        let code = Code::new(
            "
constant a : natural;
constant b : natural;
constant c : natural;
",
        );

        let mut messages = Vec::new();
        check_declarative_part_unique_ident(&code.declarative_part(), &mut messages);
        check_no_messages(&messages);
    }

    #[test]
    fn forbid_homographs() {
        let code = Code::new(
            "
constant a1 : natural;
constant a : natural;
constant a1 : natural;
",
        );

        let mut messages = Vec::new();
        check_declarative_part_unique_ident(&code.declarative_part(), &mut messages);
        assert_eq!(messages, expected_messages(&code, 1));
    }

    #[test]
    fn allows_protected_type_and_body_with_same_name() {
        let code = Code::new(
            "
type prot_t is protected
end protected;

type prot_t is protected body
end protected body;
",
        );

        let mut messages = Vec::new();
        check_declarative_part_unique_ident(&code.declarative_part(), &mut messages);
        check_no_messages(&messages);
    }

    #[test]
    fn allows_incomplete_type_definition() {
        let code = Code::new(
            "
type rec_t;
type rec_t is record
end record;
",
        );

        let mut messages = Vec::new();
        check_declarative_part_unique_ident(&code.declarative_part(), &mut messages);
        check_no_messages(&messages);
    }

    #[test]
    fn forbid_homographs_in_subprogram_bodies() {
        let code = Code::new(
            "
procedure proc(a1, a, a1 : natural) is
  constant b1 : natural;
  constant b : natural;
  constant b1 : natural;

  procedure nested_proc(c1, c, c1 : natural) is
    constant d1 : natural;
    constant d : natural;
    constant d1 : natural;
  begin
  end;

begin
end;
",
        );

        let mut messages = Vec::new();
        check_declarative_part_unique_ident(&code.declarative_part(), &mut messages);
        assert_eq!(messages, expected_messages(&code, 4));
    }

    #[test]
    fn forbid_homographs_in_component_declarations() {
        let code = Code::new(
            "
component comp is
  generic (
    a1 : natural;
    a : natural;
    a1 : natural
  );
  port (
    b1 : natural;
    b : natural;
    b1 : natural
  );
end component;
",
        );

        let mut messages = Vec::new();
        check_declarative_part_unique_ident(&code.declarative_part(), &mut messages);
        assert_eq!(messages, expected_messages(&code, 2));
    }

    #[test]
    fn forbid_homographs_in_record_type_declarations() {
        let code = Code::new(
            "
type rec_t is record
  a1 : natural;
  a : natural;
  a1 : natural;
end record;
",
        );

        let mut messages = Vec::new();
        check_declarative_part_unique_ident(&code.declarative_part(), &mut messages);
        assert_eq!(messages, expected_messages(&code, 1));
    }

    #[test]
    fn forbid_homographs_in_proteced_type_declarations() {
        let code = Code::new(
            "
type prot_t is protected
  procedure proc(a1, a, a1 : natural);
end protected;

type prot_t is protected body
  constant b1 : natural;
  constant b : natural;
  constant b1 : natural;
end protected body;
",
        );

        let mut messages = Vec::new();
        check_declarative_part_unique_ident(&code.declarative_part(), &mut messages);
        assert_eq!(messages, expected_messages(&code, 2));
    }

    #[test]
    fn forbid_homographs_in_subprogram_declarations() {
        let code = Code::new(
            "
procedure proc(a1, a, a1 : natural);
function fun(b1, a, b1 : natural) return natural;
",
        );

        let mut messages = Vec::new();
        check_declarative_part_unique_ident(&code.declarative_part(), &mut messages);
        assert_eq!(messages, expected_messages(&code, 2));
    }

    #[test]
    fn forbid_homographs_in_entity_declarations() {
        let code = Code::new(
            "
entity ent is
  generic (
    a1 : natural;
    a : natural;
    a1 : natural
  );
  port (
    b1 : natural;
    b : natural;
    b1 : natural
  );
  constant c1 : natural;
  constant c : natural;
  constant c1 : natural;
end entity;
",
        );

        let mut messages = Vec::new();
        check_entity_declaration(&code.entity(), &mut messages);
        assert_eq!(messages, expected_messages(&code, 3));
    }

}