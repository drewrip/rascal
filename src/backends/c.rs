use crate::codegen::{CodeGen, CodeGenContext, CodeGenError};
use crate::ir::{self, FuncDef, IRNode};
use crate::types::Type;
use anyhow::{bail, Error, Result};
use std::fs::File;
use std::io::Write;
use std::process::Command;

use std::collections::HashMap;

macro_rules! matches_variant {
    ($val:expr, $var:path) => {
        match $val {
            $var { .. } => true,
            _ => false,
        }
    };
}

pub fn translate_type(type_t: Type) -> String {
    match type_t {
        Type::Int32 => "int32_t",
        Type::Int64 => "int64_t",
        Type::UInt32 => "uint32_t",
        Type::UInt64 => "uint64_t",
        Type::Float32 => "float",
        Type::Float64 => "double",
        Type::Bool => "int32_t",
        Type::String => "char*",
        other => panic!("unknown type: {:?}", other),
    }
    .into()
}

pub fn translate_value(value: ir::Value) -> String {
    match value {
        ir::Value::Int32(num) => format!("INT32_C({})", num),
        ir::Value::Int64(num) => format!("INT64_C({})", num),
        ir::Value::UInt32(num) => format!("UINT32_C({})", num),
        ir::Value::UInt64(num) => format!("UINT64_C({})", num),
        ir::Value::Float32(num) => format!("{}F", num),
        ir::Value::Float64(num) => format!("{}", num),
        ir::Value::Bool(b) => {
            if b {
                format!("1")
            } else {
                format!("0")
            }
        }
        ir::Value::Id(ident) => format!("{}", ident),
        other => panic!("No value translation for: {:?}", other),
    }
}

pub fn is_expr_node(node: IRNode) -> bool {
    match node {
        IRNode::Term(_) => true,
        IRNode::Eval(_) => true,
        _ => false,
    }
}

pub struct CGenContext {
    build_stack: Vec<IRNode>,
    outfile: String,
    skip_validation: bool,
    code_buffer: Vec<String>,
}

impl From<CodeGenContext> for CGenContext {
    fn from(ctx: CodeGenContext) -> Self {
        CGenContext {
            build_stack: ctx.build_stack.into_iter().rev().collect(),
            outfile: ctx.outfile,
            skip_validation: ctx.skip_validation,
            code_buffer: vec![],
        }
    }
}

impl CodeGen for CGenContext {
    fn gen(&mut self) -> Result<(), CodeGenError> {
        self.gen_includes();
        let start = self.gen_globals();
        self.gen_program(start);

        let final_source = self.code_buffer.join(" ");
        let mut file =
            File::create("out.c").map_err(|err| CodeGenError::BinaryWrite(err.to_string()))?;
        file.write_all(final_source.as_bytes())
            .map_err(|err| CodeGenError::BinaryWrite(err.to_string()))?;

        let compile_cmd = Command::new("gcc")
            .arg("out.c")
            .arg("-o")
            .arg(self.outfile.clone())
            .output()
            .map_err(|err| CodeGenError::CompilationFailed(err.to_string()))?;

        if !compile_cmd.status.success() {
            return Err(CodeGenError::CompilationFailed(format!(
                "C compilation failed: {}",
                String::from_utf8(compile_cmd.stderr).unwrap()
            )));
        }

        Ok(())
    }
}

impl CGenContext {
    fn add_code(&mut self, code: &str) {
        self.code_buffer.push(code.into());
    }

    fn gen_includes(&mut self) -> Result<(), CodeGenError> {
        self.add_code("#include \"stdint.h\"\n");
        Ok(())
    }

    fn gen_globals(&mut self) -> usize {
        let mut idx = 0;
        // A well formed program must start with a globals section
        // which could be empty
        while (*self.build_stack.get(idx).unwrap()).clone() != IRNode::GlobalSection {
            idx += 1;
        }
        idx += 1;
        let end_of_globals = self
            .build_stack
            .iter()
            .enumerate()
            .find(|(_, ir_node)| matches_variant!(ir_node, IRNode::EndGlobalSection))
            .unwrap()
            .0;
        self.gen_code(idx, end_of_globals - 1) + 2
    }

    fn gen_program(&mut self, idx: usize) -> usize {
        self.add_code("int main(){");
        let new_idx = self.gen_code(idx, self.build_stack.len());
        self.add_code("}");
        new_idx
    }

    fn gen_code(&mut self, idx: usize, end_idx: usize) -> usize {
        let mut node_idx = idx;
        while node_idx < end_idx {
            node_idx = match self.build_stack.get(node_idx).unwrap() {
                IRNode::Term(term) => self.gen_term(node_idx).unwrap(),
                IRNode::Eval(eval) => self.gen_eval(node_idx).unwrap(),
                IRNode::Label(label) => self.gen_label(node_idx).unwrap(),
                IRNode::Assign(assign) => self.gen_assign(node_idx, assign.clone()).unwrap(),
                IRNode::Reassign(reassign) => {
                    self.gen_reassign(node_idx, reassign.clone()).unwrap()
                }
                // If Statement
                IRNode::If(if_case) => self.gen_if(node_idx).unwrap(),
                IRNode::IfCase(if_case) => self.gen_if_case(node_idx).unwrap(),
                IRNode::ElseIfCase(if_case) => self.gen_else_if_case(node_idx).unwrap(),
                IRNode::ElseCase(if_case) => self.gen_else_case(node_idx).unwrap(),
                IRNode::EndIf(if_case) => self.gen_end_if(node_idx).unwrap(),
                // Function Definitions
                IRNode::FuncDef(def, _) => self.gen_func_def(node_idx, def.clone()).unwrap(),
                IRNode::EndFuncDef(_) => self.gen_end_func_def(node_idx).unwrap(),
                // Return
                IRNode::Return => self.gen_return(node_idx).unwrap(),
                IRNode::GlobalSection => {
                    panic!("IRNode::GlobalSection should not be handled as code")
                }
                IRNode::EndGlobalSection => {
                    panic!("IRNode::EndGlobalSection should not be handled as code")
                }
                other => {
                    panic!("Unimplemented IRNode: {:?}", other);
                }
            };
        }
        node_idx
    }

    fn gen_term(&mut self, idx: usize) -> Result<usize, CodeGenError> {
        Ok(idx + 1)
    }

    fn gen_eval(&mut self, idx: usize) -> Result<usize, CodeGenError> {
        Ok(idx + 1)
    }

    fn gen_label(&mut self, idx: usize) -> Result<usize, CodeGenError> {
        Ok(idx + 1)
    }

    fn gen_assign(&mut self, idx: usize, assign: ir::Assign) -> Result<usize, CodeGenError> {
        if !matches_variant!(assign.type_t, Type::Function) {
            self.add_code(&translate_type(assign.type_t));
            self.add_code(&assign.symbol.ident.clone());
            self.add_code("=");
            self.gen_expr(idx - 1);
            self.add_code(";");
        }
        Ok(idx + 1)
    }

    fn gen_reassign(&mut self, idx: usize, reassign: ir::Reassign) -> Result<usize, CodeGenError> {
        self.add_code(&*reassign.symbol.ident.clone());
        self.add_code("=");
        self.gen_expr(idx - 1);
        self.add_code(";");
        Ok(idx + 1)
    }

    fn gen_expr(&mut self, idx: usize) -> Result<(), CodeGenError> {
        // Collect
        let expr: Vec<IRNode> = self
            .build_stack
            .iter()
            .rev()
            .skip(self.build_stack.len() - idx - 1)
            .take_while(|node| is_expr_node((*node).clone()))
            .cloned()
            .collect();

        // Use a stack to build the expression
        let mut stack: Vec<String> = vec![];
        for node in expr.into_iter().rev() {
            match node {
                IRNode::Term(term) => stack.push(translate_value(term.value)),
                IRNode::Eval(eval) => {
                    let mut sub_expr: Vec<String> = vec!["(".into()];
                    let evaluated = match eval {
                        ir::Func::Add(_) => {
                            format!("{} + {}", stack.pop().unwrap(), stack.pop().unwrap())
                        }
                        ir::Func::Sub(_) => {
                            format!("{} - {}", stack.pop().unwrap(), stack.pop().unwrap())
                        }
                        ir::Func::Mult(_) => {
                            format!("{} * {}", stack.pop().unwrap(), stack.pop().unwrap())
                        }
                        ir::Func::Div(_) => {
                            format!("{} / {}", stack.pop().unwrap(), stack.pop().unwrap())
                        }
                        ir::Func::Lt(_) => {
                            format!("{} < {}", stack.pop().unwrap(), stack.pop().unwrap())
                        }
                        ir::Func::Gt(_) => {
                            format!("{} > {}", stack.pop().unwrap(), stack.pop().unwrap())
                        }
                        ir::Func::Leq(_) => {
                            format!("{} <= {}", stack.pop().unwrap(), stack.pop().unwrap())
                        }
                        ir::Func::Geq(_) => {
                            format!("{} >= {}", stack.pop().unwrap(), stack.pop().unwrap())
                        }
                        ir::Func::Eq(_) => {
                            format!("{} == {}", stack.pop().unwrap(), stack.pop().unwrap())
                        }
                        ir::Func::Neq(_) => {
                            format!("{} != {}", stack.pop().unwrap(), stack.pop().unwrap())
                        }
                        ir::Func::Func(sig) => {
                            let mut call: String = sig.symbol.ident.clone();
                            call.push_str("(");
                            let num_params = sig.params_t.len();
                            for i in 0..num_params {
                                call.push_str(&stack.pop().unwrap());
                                if i != num_params - 1 {
                                    call.push_str(", ");
                                }
                            }
                            call.push_str(")");
                            call
                        }
                    };
                    sub_expr.push(evaluated);
                    sub_expr.push(")".into());
                    stack.push(sub_expr.join(" "))
                }
                _ => panic!("This shouldn't ever happen!"),
            };
        }
        self.add_code(&stack.pop().unwrap());
        Ok(())
    }

    fn gen_if(&mut self, idx: usize) -> Result<usize, CodeGenError> {
        Ok(idx + 1)
    }

    fn gen_if_case(&mut self, idx: usize) -> Result<usize, CodeGenError> {
        self.add_code("if");
        self.add_code("(");
        self.gen_expr(idx - 1);
        self.add_code(")");
        self.add_code("{");
        Ok(idx + 1)
    }

    fn gen_else_if_case(&mut self, idx: usize) -> Result<usize, CodeGenError> {
        self.add_code("}");
        self.add_code("else if");
        self.add_code("(");
        self.gen_expr(idx - 1);
        self.add_code(")");
        self.add_code("{");
        Ok(idx + 1)
    }

    fn gen_else_case(&mut self, idx: usize) -> Result<usize, CodeGenError> {
        self.add_code("}");
        self.add_code("else");
        self.add_code("{");
        Ok(idx + 1)
    }

    fn gen_end_if(&mut self, idx: usize) -> Result<usize, CodeGenError> {
        self.add_code("}");
        Ok(idx + 1)
    }

    fn gen_func_def(&mut self, idx: usize, def: FuncDef) -> Result<usize, CodeGenError> {
        self.add_code(&translate_type(def.return_t));
        self.add_code(&def.symbol.ident);
        self.add_code("(");
        let num_params = def.params_t.clone().len();
        for (n, param) in def.params_t.into_iter().enumerate() {
            self.add_code(&translate_type(param.1));
            self.add_code(&param.0);
            if n != num_params - 1 {
                self.add_code(",");
            }
        }
        self.add_code(")");
        self.add_code("{");
        Ok(idx + 1)
    }

    fn gen_end_func_def(&mut self, idx: usize) -> Result<usize, CodeGenError> {
        self.add_code("}");
        Ok(idx + 1)
    }

    fn gen_return(&mut self, idx: usize) -> Result<usize, CodeGenError> {
        self.add_code("return");
        self.gen_expr(idx - 1);
        self.add_code(";");
        Ok(idx + 1)
    }
}