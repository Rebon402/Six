# Six Programming Language
**A fast, secure, and sandboxed systems programming language with a serverless package manager.**

`.six` is built for high-performance isolated execution. It features AOT compilation, encrypted binaries (`.siz`), strict memory sandboxing, and a native GitHub-based package registry.

## Key Features
* **Encrypted Binaries:** Code is compiled to XOR-encrypted `.siz` files.
* **Built-in Sandbox:** Built-in CPU/Memory limits and timeouts for safe execution.
* **Serverless Registry:** Publish and install libraries directly via GitHub — no database required.
* **Signed Packages:** Cryptographic signatures prevent package tampering.
* **Reverse Debugger:** Map encrypted VM instruction pointers back to source lines safely.

## CLI Commands

```bash
six repl                  # Start the interactive REPL
six new <name>            # Create a new project structure
six build [file]          # Compile source (.six) to encrypted binary (.siz)
six run [file]            # Execute a compiled binary
six dbg <map> <IP>        # Reverse-map IP to source line (Debugger)
```

**Package Management**
```bash
six lib <file> <pkg> <v>  # Compile to a standard library
six lib-user <file>       # Compile to a user library (with signature)
six publish <pkg> <ver>   # Upload package to GitHub registry (Requires SIX_TOKEN)
six install <@pkg>        # Install a library (checks local, then GitHub)
six load-libs             # Restore dependencies from six.toml
```

## Getting Started
1. Create a project: `six new my_app`
2. Run your code: `six build src/main.six && six run release/main.siz`
3. Download a package: `six install @sys/sys_sandbox`
