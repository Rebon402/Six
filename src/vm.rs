use crate::compiler::OpCode;
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use std::collections::HashMap;
use std::time::Instant;

pub struct SafetyFrame {
    pub name: String,
    pub start_time: Instant,
    #[allow(dead_code)]
    pub allocations: usize,
}

pub struct VM {
    stack: Vec<BigInt>,
    variables: HashMap<String, BigInt>,
    heap: Vec<BigInt>, // Raw Memory Pool
    frames: Vec<SafetyFrame>,
    ip: usize,
    in_safety_frame: bool,
}

impl VM {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            variables: HashMap::new(),
            heap: vec![BigInt::from(0); 10000], // 10k slots of raw memory
            frames: Vec::new(),
            ip: 0,
            in_safety_frame: false,
        }
    }

    pub fn run(&mut self, bytecode: Vec<OpCode>) {
        let start = Instant::now();
        println!("[SixR] System Initializing...");

        unsafe {
            crate::compiler::six_guard_init(); // Initialize C Guard
        }

        while self.ip < bytecode.len() {
            let op = &bytecode[self.ip];
            match op {
                OpCode::PushInt(n) => {
                    let val = BigInt::parse_bytes(n.as_bytes(), 10).unwrap_or_default();
                    self.stack.push(val);
                }
                OpCode::PushStr(_) => {}
                OpCode::Store(name) => {
                    if let Some(val) = self.stack.pop() {
                        self.variables.insert(name.clone(), val);
                    }
                }
                OpCode::Load(name) => {
                    if let Some(val) = self.variables.get(name) {
                        self.stack.push(val.clone());
                    } else {
                        self.runtime_error(&format!("Undefined variable: {}", name));
                    }
                }
                OpCode::Addr(name) => {
                    // Raw address generation (hash-based for demonstration)
                    let addr = name.len() as i64; // Simple dummy address
                    self.stack.push(BigInt::from(addr));
                }
                OpCode::Deref => {
                    if let Some(addr) = self.stack.pop() {
                        let idx = addr.to_usize().unwrap_or(0);
                        if idx < self.heap.len() {
                            self.stack.push(self.heap[idx].clone());
                        } else {
                            self.runtime_error("Segmentation Fault: Memory out of bounds");
                        }
                    }
                }
                OpCode::StoreDeref => {
                    let addr = self.stack.pop().unwrap();
                    let val = self.stack.pop().unwrap();
                    let idx = addr.to_usize().unwrap_or(0);
                    if idx < self.heap.len() {
                        self.heap[idx] = val;
                    } else {
                        self.runtime_error("Segmentation Fault: Illegal Write");
                    }
                }
                OpCode::LeakReport => {
                    // Deprecated
                }
                OpCode::Leak => {
                    // Deprecated
                }
                OpCode::Add => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    let res = a + b;
                    // Overflow check: If not explicitly handled as BigInt, we limit to i128 for safety
                    if res > BigInt::from(i128::MAX) || res < BigInt::from(i128::MIN) {
                        self.runtime_error("Arithmetic Overflow: Operation exceeded safe bounds");
                    }
                    self.stack.push(res);
                }
                OpCode::Sub => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    let res = a - b;
                    if res > BigInt::from(i128::MAX) || res < BigInt::from(i128::MIN) {
                        self.runtime_error("Arithmetic Overflow: Operation exceeded safe bounds");
                    }
                    self.stack.push(res);
                }
                OpCode::Mul => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    let res = a * b;
                    if res > BigInt::from(i128::MAX) || res < BigInt::from(i128::MIN) {
                        self.runtime_error("Arithmetic Overflow: Operation exceeded safe bounds");
                    }
                    self.stack.push(res);
                }
                OpCode::Div => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    if b == BigInt::from(0) {
                        self.runtime_error("Division by zero");
                    }
                    self.stack.push(a / b);
                }
                OpCode::Xor => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a ^ b);
                }
                OpCode::Or => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a | b);
                }
                OpCode::Jump(target) => {
                    self.ip = *target;
                    continue;
                }
                OpCode::JumpIfFalse(target) => {
                    if let Some(val) = self.stack.pop() {
                        if val == BigInt::from(0) {
                            self.ip = *target;
                            continue;
                        }
                    }
                }
                OpCode::Put => {
                    if self.in_safety_frame {
                        self.runtime_error("Safety Violation: External Write forbidden inside try block");
                    }
                    if let Some(val) = self.stack.pop() {
                        println!("{}", val);
                    }
                }
                OpCode::EnterTry => {
                    self.in_safety_frame = true;
                    self.frames.push(SafetyFrame {
                        name: format!("Frame_{}", self.frames.len()),
                        start_time: Instant::now(),
                        allocations: 0,
                    });
                }
                OpCode::ExitTry => {
                    if let Some(_) = self.frames.pop() {
                        if self.frames.is_empty() {
                            self.in_safety_frame = false;
                        }
                    }
                }
                OpCode::ArenaStart => {
                    // C core handles the single global arena for now
                    // In a multi-threaded VM we'd have thread-local arenas
                }
                OpCode::ArenaEnd => unsafe {
                    crate::compiler::six_arena_clear();
                },
                OpCode::Halt => break,
                _ => {}
            }

            // Security Heartbeat Check (Zig & C)
            if self.ip % 100 == 0 {
                unsafe {
                    if crate::compiler::six_security_heartbeat() != 0
                        || crate::compiler::six_guard_heartbeat() != 0
                    {
                        println!("[SixR SECURITY] CRITICAL THREAT: Debugger/Tamper Found!");
                        crate::compiler::six_arena_clear(); // Clear memory immediately
                        crate::compiler::six_guard_lock_release();
                        std::process::exit(0xDEAD); // Hardcore Crash
                    }
                }
            }

            self.ip += 1;
        }
        let total_duration = start.elapsed();
        println!(
            "\n[SixR] Execution finished in {:.6}s",
            total_duration.as_secs_f64()
        );
    }

    fn runtime_error(&self, msg: &str) {
        eprintln!("\n[SixR ERROR] {}", msg);
        eprintln!("Instruction Pointer: {}", self.ip);
        if let Some(frame) = self.frames.last() {
            eprintln!("Active Safety Frame: {}", frame.name);
            eprintln!(
                "Frame Uptime: {:.6}s",
                frame.start_time.elapsed().as_secs_f64()
            );
        }
        unsafe {
            crate::compiler::six_guard_lock_release();
        }
        std::process::exit(1);
    }
}
