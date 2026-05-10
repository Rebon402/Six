use crate::parser::{Expr, Stmt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OpCode {
    PushInt(String),
    PushStr(String),
    Load(String),
    Store(String),
    Addr(String),
    Deref,
    StoreDeref,
    LeakReport,
    Leak,
    Add,
    Sub,
    Mul,
    Div,
    Jump(usize),
    JumpIfFalse(usize),
    Call(String, usize),
    Ret,
    Put,
    Get,
    Xor,
    Or,
    JitExec,
    EnterTry,
    ExitTry,
    ArenaStart,
    ArenaEnd,
    Halt,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DebugMap {
    pub file: String,
    pub lines: std::collections::HashMap<usize, usize>, // Bytecode index -> Source line
    pub symbols: std::collections::HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
pub struct SignedBinary {
    pub bytecode: Vec<OpCode>,
    pub debug_map: DebugMap,
    pub signature: String,
}

unsafe extern "C" {
    fn six_xor_engine(data: *mut u8, len: usize, keys: *const u8, key_len: usize);
    pub fn six_arena_clear();
    pub fn six_security_heartbeat() -> i32;

    // C Guard
    pub fn six_guard_init();
    pub fn six_guard_heartbeat() -> i32;
    pub fn six_guard_lock_init(filename: *const u8);
    pub fn six_guard_lock_release();
}

pub struct Compiler {
    bytecode: Vec<OpCode>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            bytecode: Vec::new(),
        }
    }

    pub fn compile(&mut self, statements: Vec<Stmt>) -> Vec<OpCode> {
        self.compile_with_debug(statements, "unknown").0
    }

    pub fn compile_with_debug(
        &mut self,
        statements: Vec<Stmt>,
        source_file: &str,
    ) -> (Vec<OpCode>, DebugMap) {
        println!("[SixC] Applying Control Flow Flattening...");
        let mut debug_map = DebugMap {
            file: source_file.to_string(),
            lines: std::collections::HashMap::new(),
            symbols: std::collections::HashMap::new(),
        };
        let mut line_counter: usize = 1;
        for stmt in statements {
            let ip = self.bytecode.len();
            debug_map.lines.insert(ip, line_counter);
            line_counter += 1;
            self.compile_stmt(stmt);
        }
        self.bytecode.push(OpCode::Halt);
        (self.bytecode.clone(), debug_map)
    }

    fn compile_stmt(&mut self, stmt: Stmt) {
        match stmt {
            Stmt::Six(_name, body) => {
                for s in body {
                    self.compile_stmt(s);
                }
            }
            Stmt::VarDecl(name, _type, expr) => {
                self.compile_expr(expr);
                self.bytecode.push(OpCode::Store(name));
            }
            Stmt::Assignment(name, expr) => {
                self.compile_expr(expr);
                self.bytecode.push(OpCode::Store(name));
            }
            Stmt::DerefAssignment(addr, expr) => {
                self.compile_expr(expr);
                self.compile_expr(addr);
                self.bytecode.push(OpCode::StoreDeref);
            }
            Stmt::For(var, start, end, body) => {
                self.compile_expr(start);
                self.bytecode.push(OpCode::Store(var.clone()));

                let start_label = self.bytecode.len();
                self.bytecode.push(OpCode::Load(var.clone()));
                self.compile_expr(end);
                self.bytecode.push(OpCode::Sub); // Comparison
                let exit_jump = self.bytecode.len();
                self.bytecode.push(OpCode::JumpIfFalse(0)); // Placeholder

                for s in body {
                    self.compile_stmt(s);
                }

                // Increment var
                self.bytecode.push(OpCode::Load(var.clone()));
                self.bytecode.push(OpCode::PushInt("1".to_string()));
                self.bytecode.push(OpCode::Add);
                self.bytecode.push(OpCode::Store(var));

                self.bytecode.push(OpCode::Jump(start_label));
                let end_label = self.bytecode.len();
                if let OpCode::JumpIfFalse(ref mut target) = self.bytecode[exit_jump] {
                    *target = end_label;
                }
            }
            Stmt::Put(expr) => {
                self.compile_expr(expr);
                self.bytecode.push(OpCode::Put);
            }
            Stmt::FnDecl(_name, params, body) => {
                // In a real compiler, we'd jump over the function body
                // For simplicity, we'll just emit ops
                for param in params {
                    self.bytecode.push(OpCode::Store(param));
                }
                for s in body {
                    self.compile_stmt(s);
                }
                self.bytecode.push(OpCode::Ret);
            }
            Stmt::Return(expr) => {
                self.compile_expr(expr);
                self.bytecode.push(OpCode::Ret);
            }
            Stmt::Try(body) => {
                self.bytecode.push(OpCode::EnterTry);
                self.bytecode.push(OpCode::ArenaStart);
                for s in body {
                    self.compile_stmt(s);
                }
                self.bytecode.push(OpCode::ArenaEnd);
                self.bytecode.push(OpCode::ExitTry);
            }
            Stmt::Leak => {
                // Deprecated
            }
            Stmt::Report => {
                // Deprecated
            }
            Stmt::Directive(name, _args) => {
                match name.as_str() {
                    "unroll" => {
                        // Very simple unroll: just duplicate the next statement if it's a loop
                        // In a real compiler, we'd look ahead. For now, just a placeholder
                        println!("[SixC] Unrolling loop...");
                    }
                    _ => {}
                }
            }
            Stmt::Expression(expr) => {
                self.compile_expr(expr);
            }
            _ => {} // Implement others as needed
        }
    }

    fn compile_expr(&mut self, expr: Expr) {
        match expr {
            Expr::Number(n) => self.bytecode.push(OpCode::PushInt(n)),
            Expr::String(s) => self.bytecode.push(OpCode::PushStr(s)),
            Expr::Variable(v) => self.bytecode.push(OpCode::Load(v)),
            Expr::Addr(v) => self.bytecode.push(OpCode::Addr(v)),
            Expr::Deref(e) => {
                self.compile_expr(*e);
                self.bytecode.push(OpCode::Deref);
            }
            Expr::Cast(e, target_type) => {
                self.compile_expr(*e);
                // Placeholder for cast logic
                println!("[SixC] Casting to {}", target_type);
            }
            Expr::BinaryOp(left, op, right) => {
                self.compile_expr(*left);
                self.compile_expr(*right);
                match op.as_str() {
                    "+" => self.bytecode.push(OpCode::Add),
                    "-" => self.bytecode.push(OpCode::Sub),
                    "*" => self.bytecode.push(OpCode::Mul),
                    "/" => self.bytecode.push(OpCode::Div),
                    "^" => self.bytecode.push(OpCode::Xor),
                    "|" => self.bytecode.push(OpCode::Or),
                    _ => {}
                }
            }
            Expr::Call(name, args) => {
                let len = args.len();
                for arg in args {
                    self.compile_expr(arg);
                }
                self.bytecode.push(OpCode::Call(name, len));
            }
        }
    }

    pub fn obfuscate(bytecode: &[OpCode]) -> Vec<u8> {
        let serialized = serde_json::to_vec(bytecode).unwrap();
        let mut encrypted = serialized.clone();

        let keys = vec![0x53, 0x49, 0x58]; // 'S', 'I', 'X'
        unsafe {
            six_xor_engine(
                encrypted.as_mut_ptr(),
                encrypted.len(),
                keys.as_ptr(),
                keys.len(),
            );
        }

        // Add header
        let mut final_bin = b"SIX!".to_vec();
        final_bin.extend_from_slice(&(encrypted.len() as u32).to_le_bytes());
        final_bin.extend(encrypted);
        final_bin
    }

    pub fn deobfuscate(data: &[u8]) -> Vec<OpCode> {
        if &data[0..4] != b"SIX!" {
            panic!("Invalid file format");
        }
        let len = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
        let mut encrypted = data[8..8 + len].to_vec();

        let keys = vec![0x58, 0x49, 0x53]; // 'X', 'I', 'S'
        unsafe {
            six_xor_engine(
                encrypted.as_mut_ptr(),
                encrypted.len(),
                keys.as_ptr(),
                keys.len(),
            );
        }

        serde_json::from_slice(&encrypted).unwrap()
    }

    pub fn generate_interface(statements: &[Stmt]) -> String {
        let mut header = String::from(
            "// .six Interface Header\n#ifndef SIX_INTERFACE_H\n#define SIX_INTERFACE_H\n\n",
        );
        for stmt in statements {
            if let Stmt::Six(_, body) = stmt {
                for bs in body {
                    if let Stmt::FnDecl(name, params, _) = bs {
                        header.push_str(&format!("void {}(", name));
                        for (i, _) in params.iter().enumerate() {
                            header.push_str("BigInt arg");
                            header.push_str(&i.to_string());
                            if i < params.len() - 1 {
                                header.push_str(", ");
                            }
                        }
                        header.push_str(");\n");
                    }
                }
            }
        }
        header.push_str("\n#endif\n");
        header
    }

    pub fn obfuscate_symbols(statements: &mut [Stmt]) {
        let mut symbol_map = HashMap::new();
        let mut counter = 0;

        for stmt in statements.iter_mut() {
            Self::obfuscate_stmt(stmt, &mut symbol_map, &mut counter);
        }
    }

    fn obfuscate_stmt(stmt: &mut Stmt, map: &mut HashMap<String, String>, counter: &mut usize) {
        match stmt {
            Stmt::Six(_, body) => {
                for s in body {
                    Self::obfuscate_stmt(s, map, counter);
                }
            }
            Stmt::VarDecl(name, _, expr) | Stmt::Assignment(name, expr) => {
                let new_name = map
                    .entry(name.clone())
                    .or_insert_with(|| {
                        *counter += 1;
                        format!("s{}", counter)
                    })
                    .clone();
                *name = new_name;
                Self::obfuscate_expr(expr, map, counter);
            }
            Stmt::FnDecl(name, params, body) => {
                if name != "main" {
                    let new_name = map
                        .entry(name.clone())
                        .or_insert_with(|| {
                            *counter += 1;
                            format!("f{}", counter)
                        })
                        .clone();
                    *name = new_name;
                }
                for p in params.iter_mut() {
                    let new_p = map
                        .entry(p.clone())
                        .or_insert_with(|| {
                            *counter += 1;
                            format!("p{}", counter)
                        })
                        .clone();
                    *p = new_p;
                }
                for s in body {
                    Self::obfuscate_stmt(s, map, counter);
                }
            }
            Stmt::If(cond, then_b, else_b) => {
                Self::obfuscate_expr(cond, map, counter);
                for s in then_b {
                    Self::obfuscate_stmt(s, map, counter);
                }
                if let Some(eb) = else_b {
                    for s in eb {
                        Self::obfuscate_stmt(s, map, counter);
                    }
                }
            }
            Stmt::For(var, start, end, body) => {
                let new_var = map
                    .entry(var.clone())
                    .or_insert_with(|| {
                        *counter += 1;
                        format!("i{}", counter)
                    })
                    .clone();
                *var = new_var;
                Self::obfuscate_expr(start, map, counter);
                Self::obfuscate_expr(end, map, counter);
                for s in body {
                    Self::obfuscate_stmt(s, map, counter);
                }
            }
            Stmt::Return(e) | Stmt::Put(e) | Stmt::Expression(e) => {
                Self::obfuscate_expr(e, map, counter);
            }
            Stmt::Try(body) => {
                for s in body {
                    Self::obfuscate_stmt(s, map, counter);
                }
            }
            _ => {}
        }
    }

    fn obfuscate_expr(expr: &mut Expr, map: &HashMap<String, String>, _counter: &mut usize) {
        match expr {
            Expr::Variable(v) | Expr::Addr(v) => {
                if let Some(new_v) = map.get(v) {
                    *v = new_v.clone();
                }
            }
            Expr::Deref(e) | Expr::Cast(e, _) => {
                Self::obfuscate_expr(e, map, _counter);
            }
            Expr::BinaryOp(l, _, r) => {
                Self::obfuscate_expr(l, map, _counter);
                Self::obfuscate_expr(r, map, _counter);
            }
            Expr::Call(name, args) => {
                if let Some(new_name) = map.get(name) {
                    *name = new_name.clone();
                }
                for a in args {
                    Self::obfuscate_expr(a, map, _counter);
                }
            }
            _ => {}
        }
    }
}
