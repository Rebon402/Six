mod compiler;
mod lexer;
mod parser;
mod vm;

use base64;
use compiler::{Compiler, SignedBinary};
use lexer::Lexer;
use parser::Parser;
use serde_json;
use std::env;
use std::fs;
use std::path::Path;
use vm::VM;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Six Language Toolchain v1.0.1");
        println!("Usage:");
        println!("  six repl               - Interactive REPL");
        println!("  six new <name>         - Create new project");
        println!("  six build [file]       - Compile .six -> .siz");
        println!("  six run [file]         - Run .siz binary");
        println!("  six lib <file>         - Compile to library");
        println!("  six install <@pkg>     - Install a library");
        println!("  six load-libs          - Restore from six.toml");
        println!("  six dbg <map> <IP>     - Reverse-map IP to source line");
        return;
    }

    let command = &args[1];

    // REPL — safe against EOF / piped stdin
    if command == "repl" {
        use std::io::{BufRead, Write};
        println!("Six REPL v1.0.1  (type 'exit' or Ctrl+D to quit)");
        let stdin = std::io::stdin();
        let mut vm = VM::new();
        loop {
            print!("six> ");
            std::io::stdout().flush().unwrap();
            let mut input = String::new();
            match stdin.lock().read_line(&mut input) {
                Ok(0) | Err(_) => break, // EOF
                _ => {}
            }
            let input = input.trim();
            if input.is_empty() {
                continue;
            }
            if input == "exit" {
                break;
            }
            let source = format!("six REPL\n    fn main()\n        {}\n    end\nend", input);
            let tokens = Lexer::new(&source).tokenize();
            let mut parser = Parser::new(tokens, source.clone());
            let stmts = parser.parse();
            if !stmts.is_empty() {
                let bytecode = Compiler::new().compile(stmts);
                vm.run(bytecode);
            }
        }
        return;
    }

    match command.as_str() {
        "dbg" => {
            if args.len() < 4 {
                println!("Usage: six dbg <file.map.siz> <IP>");
                return;
            }
            let map_path = &args[2];
            let ip: usize = args[3].parse().unwrap_or(0);
            let map_content = fs::read_to_string(map_path).expect("Failed to read map");

            // Map file is a SignedBinary JSON — extract the debug_map field
            let signed: SignedBinary = serde_json::from_str(&map_content)
                .expect("Invalid map format (expected SignedBinary)");
            let map = &signed.debug_map;

            if let Some(line) = map.lines.get(&ip) {
                println!("[SixD] IP:{} -> {}:{}", ip, map.file, line);
                println!("[SixD] Signature: {}", signed.signature);
            } else {
                println!("[SixD ERROR] No mapping for IP:{}", ip);
            }
            return;
        }
        "new" => {
            if args.len() < 3 {
                println!("Usage: six new <name>");
                return;
            }
            let name = &args[2];
            let root = format!("{}", name);
            let src = format!("{}/src", name);

            std::fs::create_dir_all(&src).unwrap();

            let read_template = |path: &str, default: &str| -> String {
                std::fs::read_to_string(format!("template/{}", path))
                    .map(|s| s.replace("{{name}}", name))
                    .unwrap_or_else(|_| default.replace("{{name}}", name))
            };

            // Create src/main.six
            let main_six = read_template(
                "main.six",
                "six {{name}}\n    fn main()\n        put \"Hello, Six!\"\n    end\nend\n",
            );
            std::fs::write(format!("{}/main.six", src), main_six).unwrap();

            // Create .gitattributes
            let git_attr = read_template(
                ".gitattributes",
                "*.six text\n*.siz binary\n*.siz linguist-vendored\n",
            );
            std::fs::write(format!("{}/.gitattributes", root), git_attr).unwrap();

            // Create .gitignore
            let git_ignore = read_template(
                ".gitignore",
                "/target\n/release\n*.siz\n*.siz.lib\n*.six.h\n",
            );
            std::fs::write(format!("{}/.gitignore", root), git_ignore).unwrap();

            // Create six.toml
            let six_toml = read_template(
                "six.toml",
                "[project]\nname = \"{{name}}\"\nversion = \"0.1.0\"\ntype = \"bin\"\n",
            );
            std::fs::write(format!("{}/six.toml", root), six_toml).unwrap();

            println!(
                "[SixC] Created new project structure for: {} (from templates)",
                name
            );
        }
        "init" => {
            std::fs::create_dir_all("src").unwrap();
            std::fs::write(
                "src/main.six",
                "six MyProject\n    fn main()\n        put \"Hello, Six!\"\n    end\nend\n",
            )
            .unwrap();
            println!("[SixC] Initialized Six project in current directory");
        }
        "tokens" => {
            let source = std::fs::read_to_string(&args[2]).expect("Failed to read file");
            let mut lexer = Lexer::new(&source);
            let tokens = lexer.tokenize();
            for t in tokens {
                println!("{:?}", t);
            }
        }
        "build" => {
            let input_path = if args.len() < 3 {
                "src/main.six"
            } else {
                &args[2]
            };
            let source = fs::read_to_string(input_path).expect("Failed to read source");
            println!("[SixC] Compiling {}...", input_path);

            let tokens = Lexer::new(&source).tokenize();
            let mut parser = Parser::new(tokens, source.clone());
            let ast = parser.parse();

            let mut compiler = Compiler::new();
            let (bytecode, debug_map) = compiler.compile_with_debug(ast, input_path);

            let encrypted = Compiler::obfuscate(&bytecode);

            // Generate a simple XOR-based checksum signature
            let signature: u64 = encrypted
                .iter()
                .enumerate()
                .fold(0u64, |acc, (i, &b)| acc ^ ((b as u64) << (i % 8)));
            let sig_hex = format!("{:016X}", signature);

            let signed = SignedBinary {
                bytecode: bytecode.clone(),
                debug_map: debug_map.clone(),
                signature: sig_hex.clone(),
            };

            let stem = Path::new(input_path).file_stem().unwrap().to_str().unwrap();
            let output_siz = format!("release/{}.siz", stem);
            let output_map = format!("other/map/{}.map.siz", stem);

            fs::create_dir_all("release").ok();
            fs::create_dir_all("other/map").ok();
            fs::write(&output_siz, &encrypted).expect("Failed to write .siz");
            fs::write(&output_map, serde_json::to_string_pretty(&signed).unwrap())
                .expect("Failed to write .map.siz");

            println!("[SixC] Success! Generated:");
            println!("  -> {} (signature: {})", output_siz, sig_hex);
            println!("  -> {} (debug map)", output_map);
        }
        "install" => {
            if args.len() < 3 {
                println!("Usage: six install <@scope/name>");
                return;
            }
            let package = &args[2];
            println!("[SixC] Calling Six Installer (sixi) for {}...", package);

            let status = std::process::Command::new("./sixi.exe")
                .arg("install")
                .arg(package)
                .status()
                .expect("Failed to execute sixi.exe");

            if !status.success() {
                println!("[SixC ERROR] Installation failed.");
            }
        }
        "load-libs" => {
            println!("[SixC] Restoring project dependencies...");
            let status = std::process::Command::new("./sixi.exe")
                .arg("load-libs")
                .arg("")
                .status()
                .expect("Failed to execute sixi.exe");

            if !status.success() {
                println!("[SixC ERROR] Failed to load libraries.");
            }
        }
        "lib" => {
            let input_path = if args.len() < 3 {
                "src/main.six"
            } else {
                &args[2]
            };
            let source = fs::read_to_string(input_path).expect("Failed to read source");

            let (lib_name, version) = if args.len() >= 5 {
                (args[3].clone(), args[4].clone())
            } else {
                use std::io::{self, Write};
                print!("[SixC] Enter Library Name (e.g., @name/lib): ");
                io::stdout().flush().unwrap();
                let mut ln = String::new();
                io::stdin().read_line(&mut ln).unwrap();

                print!("[SixC] Enter Version (e.g., v0.0.1): ");
                io::stdout().flush().unwrap();
                let mut v = String::new();
                io::stdin().read_line(&mut v).unwrap();

                (ln.trim().to_string(), v.trim().to_string())
            };

            let sanitized_name = lib_name.replace("@", "").replace("/", "-");
            let final_filename = format!("{}-{}", sanitized_name, version);

            let mut lexer = Lexer::new(&source);
            let tokens = lexer.tokenize();
            let mut parser = Parser::new(tokens, source.clone());
            let mut ast = parser.parse();

            Compiler::obfuscate_symbols(&mut ast);

            let mut compiler = Compiler::new();
            let bytecode = compiler.compile(ast.clone());

            let encrypted = Compiler::obfuscate(&bytecode);
            let header = Compiler::generate_interface(&ast);

            let output_dir = "other/libs";
            fs::create_dir_all(output_dir).ok();

            let output_lib = format!("{}/{}.siz.lib", output_dir, final_filename);
            let output_h = format!("{}/{}.six.h", output_dir, final_filename);

            fs::write(&output_lib, encrypted).expect("Failed to write .siz.lib");
            fs::write(&output_h, header).expect("Failed to write .six.h");

            println!("[SixC] Library Success! Deployed to {}:", output_dir);
            println!("  -> {}", output_lib);
            println!("  -> {}", output_h);
        }
        "run" => {
            let input_path = if args.len() < 3 {
                "release/main.siz"
            } else {
                &args[2]
            };
            let data = fs::read(input_path).expect("Failed to read .siz");

            println!("[SixR] Loading {}...", input_path);
            let bytecode = Compiler::deobfuscate(&data);

            unsafe {
                let c_path = std::ffi::CString::new(input_path).unwrap();
                crate::compiler::six_guard_lock_init(c_path.as_ptr() as *const u8);
            }

            let mut vm = VM::new();
            vm.run(bytecode);

            unsafe {
                crate::compiler::six_guard_lock_release();
            }
        }
        "lib-user" => {
            let input_path = if args.len() < 3 {
                "src/main.six"
            } else {
                &args[2]
            };
            let source = fs::read_to_string(input_path).expect("Failed to read source");

            let (lib_name, version) = if args.len() >= 5 {
                (args[3].clone(), args[4].clone())
            } else {
                use std::io::{self, Write};
                print!("[SixC] User Library Name (e.g., @yourname/lib): ");
                io::stdout().flush().unwrap();
                let mut ln = String::new();
                io::stdin().read_line(&mut ln).unwrap();
                print!("[SixC] Version (e.g., v1.0.0): ");
                io::stdout().flush().unwrap();
                let mut v = String::new();
                io::stdin().read_line(&mut v).unwrap();
                (ln.trim().to_string(), v.trim().to_string())
            };

            let sanitized_name = lib_name.replace("@", "").replace("/", "-");
            let final_filename = format!("{}-{}", sanitized_name, version);

            let tokens = Lexer::new(&source).tokenize();
            let mut parser = Parser::new(tokens, source.clone());
            let mut ast = parser.parse();
            Compiler::obfuscate_symbols(&mut ast);

            let mut compiler = Compiler::new();
            let bytecode = compiler.compile(ast.clone());
            let encrypted = Compiler::obfuscate(&bytecode);

            // Compute and embed signature into the header
            let sig: u64 = encrypted
                .iter()
                .enumerate()
                .fold(0u64, |acc, (i, &b)| acc ^ ((b as u64) << (i % 8)));
            let sig_hex = format!("{:016X}", sig);

            let mut header = Compiler::generate_interface(&ast);
            header.push_str(&format!("\n// SIG:{}\n", sig_hex));
            header.push_str(&format!("// AUTHOR:{}\n", lib_name));
            header.push_str(&format!("// VERSION:{}\n", version));

            let output_dir = "other/user_libs";
            fs::create_dir_all(output_dir).ok();

            let output_lib = format!("{}/{}.siz.lib", output_dir, final_filename);
            let output_h = format!("{}/{}.six.h", output_dir, final_filename);

            fs::write(&output_lib, &encrypted).expect("Failed to write .siz.lib");
            fs::write(&output_h, &header).expect("Failed to write .six.h");

            println!("[SixC] User Library Deployed to {}:", output_dir);
            println!("  -> {} (signed: {})", output_lib, sig_hex);
            println!("  -> {}", output_h);
        }
        "publish" => {
            // six publish @Scope/name version
            // Uploads to github.com/Rebon402/six-registry  (no DB — Git is the registry)
            if args.len() < 4 {
                println!("Usage: six publish <@scope/name> <version>");
                println!("  Set SIX_TOKEN env var to your GitHub personal access token.");
                return;
            }
            let pkg     = &args[2]; // e.g. @Rebon402/sandbox-vm
            let version = &args[3]; // e.g. v1.0.0
            let token   = std::env::var("SIX_TOKEN").unwrap_or_default();
            if token.is_empty() {
                println!("[SixP ERROR] SIX_TOKEN not set. Export your GitHub PAT first.");
                return;
            }

            let sanitized = pkg.replace('@', "").replace('/', "-");
            let stem      = format!("{}-{}", sanitized, version);
            let lib_path  = format!("other/user_libs/{}.siz.lib", stem);
            let hdr_path  = format!("other/user_libs/{}.six.h",   stem);

            for (local, remote_name) in [
                (&lib_path, format!("{}.siz.lib", stem)),
                (&hdr_path, format!("{}.six.h",   stem)),
            ] {
                let data = match fs::read(local) {
                    Ok(d) => d,
                    Err(_) => { println!("[SixP ERROR] File not found: {}", local); return; }
                };
                let encoded = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD, &data
                );

                let api_url = format!(
                    "https://api.github.com/repos/Rebon402/six-registry/contents/libs/{}",
                    remote_name
                );

                // Check if file already exists (need SHA for update)
                let sha: Option<String> = ureq::get(&api_url)
                    .set("Authorization", &format!("token {}", token))
                    .set("User-Agent", "six-cli")
                    .call()
                    .ok()
                    .and_then(|r| r.into_json::<serde_json::Value>().ok())
                    .and_then(|v| v["sha"].as_str().map(|s| s.to_string()));

                let mut body = serde_json::json!({
                    "message": format!("publish {} {}", pkg, version),
                    "content": encoded
                });
                if let Some(s) = sha {
                    body["sha"] = serde_json::Value::String(s);
                }

                let res = ureq::put(&api_url)
                    .set("Authorization", &format!("token {}", token))
                    .set("User-Agent", "six-cli")
                    .send_json(&body);

                match res {
                    Ok(_)  => println!("[SixP] ✓ Uploaded: {}", remote_name),
                    Err(e) => println!("[SixP ERROR] {}: {:?}", remote_name, e),
                }
            }
            println!("[SixP] Published {} {} to Rebon402/six-registry", pkg, version);
        }
        _ => println!("Unknown command. Run 'six' for usage."),
    }
}
