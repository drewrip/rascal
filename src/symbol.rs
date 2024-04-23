use std::sync::Arc;

use crate::ast;
use crate::types;

#[derive(Debug, Clone)]
pub struct IdentMapping {
    pub symbol: Symbol,
    pub var: Var,
}

pub fn new_identmapping(symbol: Symbol, var: Var) -> IdentMapping {
    IdentMapping { symbol, var }
}

#[derive(Debug, Clone)]
pub struct Var {
    pub type_t: types::Type,
    pub node: ast::Node,
}

pub fn new_var(type_t: types::Type, node: ast::Node) -> Var {
    Var { type_t, node }
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct Symbol {
    pub ident: String,
}

pub fn new_symbol(ident: String) -> Symbol {
    Symbol { ident }
}

pub trait Symbolic {
    fn get_symbol(&self) -> Option<IdentMapping>;
}

impl Symbolic for ast::Program {
    fn get_symbol(&self) -> Option<IdentMapping> {
        match self {
            ast::Program::NoWith(ident, block) => Some(IdentMapping {
                symbol: ident.clone(),
                var: Var {
                    type_t: types::Type::Program(types::ProgramType { with_t: vec![] }),
                    node: ast::Node::BlockNode(Arc::new(block.clone())),
                },
            }),
            ast::Program::With(ident, with_vars, block) => Some(IdentMapping {
                symbol: ident.clone(),
                var: Var {
                    type_t: types::Type::Program(types::ProgramType {
                        with_t: with_vars
                            .iter()
                            .map(|with_var| match *with_var.clone() {
                                ast::WithVar::Imm(_) => types::WithType::Imm,
                                ast::WithVar::Mut(_) => types::WithType::Mut,
                            })
                            .collect(),
                    }),
                    node: ast::Node::BlockNode(Arc::new(block.to_vec())),
                },
            }),
        }
    }
}

impl Symbolic for ast::Stmt {
    fn get_symbol(&self) -> Option<IdentMapping> {
        match self {
            ast::Stmt::Assign(symbol, var, expr) => Some(IdentMapping {
                symbol: symbol.clone(),
                var: (*var.clone()).clone(),
            }),
            ast::Stmt::FuncDef(func) => Some(IdentMapping {
                symbol: Symbol {
                    ident: func.ident.clone(),
                },
                var: Var {
                    type_t: types::Type::Function(types::FunctionType {
                        return_t: vec![func.ret_t.clone()],
                        params_t: func
                            .params
                            .iter()
                            .map(|param| param.type_t.clone())
                            .collect(),
                        with_t: func
                            .with
                            .iter()
                            .map(|with_var| match *with_var.clone() {
                                ast::WithVar::Imm(_) => types::WithType::Imm,
                                ast::WithVar::Mut(_) => types::WithType::Mut,
                            })
                            .collect(),
                    }),
                    node: ast::Node::BlockNode(Arc::new(func.block.clone())),
                },
            }),
            _ => None,
        }
    }
}