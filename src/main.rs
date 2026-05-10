mod compiler;
mod lexer;
mod parser;
mod vm;

use compiler::Compiler;
use lexer::Lexer;
use parser::Parser;
use std::env;
use std::fs;
use std::path::Path;
use vm::VM;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage:");
        println!("  six build <file.six>");
        println!("  six run <file.siz>");
        return;
    }

    let command = &args[1];
    match command.as_str() {
        "tokens" => {
            let source = std::fs::read_to_string(&args[2]).expect("Failed to read file");
            let mut lexer = Lexer::new(&source);
            let tokens = lexer.tokenize();
            for t in tokens {
                println!("{:?}", t);
            }
        }
        "build" => {
            if args.len() < 3 {
                return;
            }
            let input_path = &args[2];
            let source = fs::read_to_string(input_path).expect("Failed to read source");

            println!("[SixC] Compiling {}...", input_path);
            let mut lexer = Lexer::new(&source);
            let mut tokens = Vec::new();
            loop {
                let data = lexer.next_token();
                if data.token == lexer::Token::EOF {
                    break;
                }
                tokens.push(data);
            }

            let mut parser = Parser::new(tokens, source.clone());
            let ast = parser.parse();

            let mut compiler = Compiler::new();
            let bytecode = compiler.compile(ast);

            let encrypted = Compiler::obfuscate(&bytecode);

            let stem = Path::new(input_path).file_stem().unwrap().to_str().unwrap();
            let output_siz = format!("release/{}.siz", stem);
            let output_map = format!("other/map/{}.map.siz", stem);

            fs::write(&output_siz, encrypted).expect("Failed to write .siz");

            // Map file is a JSON version for "human readable" debugging as requested
            let map_json = serde_json::to_string_pretty(&bytecode).unwrap();
            fs::write(&output_map, map_json).expect("Failed to write .map.siz");

            println!("[SixC] Success! Generated:");
            println!("  -> {}", output_siz);
            println!("  -> {}", output_map);
        }
        "lib" => {
            if args.len() < 3 {
                return;
            }
            let input_path = &args[2];
            let source = fs::read_to_string(input_path).expect("Failed to read source");

            println!("[SixC] Generating Library for {}...", input_path);
            let mut lexer = Lexer::new(&source);
            let mut tokens = Vec::new();
            loop {
                let data = lexer.next_token();
                if data.token == lexer::Token::EOF {
                    break;
                }
                tokens.push(data);
            }

            let mut parser = Parser::new(tokens, source.clone());
            let mut ast = parser.parse();

            Compiler::obfuscate_symbols(&mut ast);

            let mut compiler = Compiler::new();
            let bytecode = compiler.compile(ast.clone());

            let encrypted = Compiler::obfuscate(&bytecode);
            let header = Compiler::generate_interface(&ast);

            let stem = Path::new(input_path).file_stem().unwrap().to_str().unwrap();
            let output_lib = format!("release/libs/{}.siz.lib", stem);
            let output_h = format!("release/libs/{}.six.h", stem);

            fs::create_dir_all("release/libs").ok();
            fs::write(&output_lib, encrypted).expect("Failed to write .siz.lib");
            fs::write(&output_h, header).expect("Failed to write .six.h");

            println!("[SixC] Library Success! Generated:");
            println!("  -> {}", output_lib);
            println!("  -> {}", output_h);
        }
        "run" => {
            if args.len() < 3 {
                return;
            }
            let input_path = &args[2];
            let data = fs::read(input_path).expect("Failed to read .siz");

            println!("[SixR] Loading {}...", input_path);
            let bytecode = Compiler::deobfuscate(&data);

            unsafe {
                let c_path = std::ffi::CString::new(input_path.as_str()).unwrap();
                crate::compiler::six_guard_lock_init(c_path.as_ptr() as *const u8);
            }

            let mut vm = VM::new();
            vm.run(bytecode);

            unsafe {
                crate::compiler::six_guard_lock_release();
            }
        }
        _ => println!("Unknown command"),
    }
}
