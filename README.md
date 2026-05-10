# 🛡️ .six Programming Language
**The Hardcore Systems Programming Environment for Security-Critical Applications.**

`.six` is a specialized, production-hardened programming language designed for high-performance execution within a strictly sandboxed environment. It combines the power of Rust, Zig, and C to create a "Zero-Leak" ecosystem ideal for hosting sensitive logic, such as crypto engines, anti-cheat systems, or isolated bot services.

## 🚀 Key Features

### 1. Hardcore Security & Anti-Tamper
- **No-JIT Execution**: Strictly AOT (Ahead-Of-Time) compiled to prevent runtime machine code modification.
- **Zero Disk Decryption**: Code is XOR-Rolling encrypted and decrypted directly into RAM. No unencrypted traces are ever left on disk.
- **Anti-Debug Panic**: Integrated hardware heartbeat that crashes the VM (Exit Code `0xDEAD`) if a debugger or memory tamper is detected.
- **Single Instance Lock**: Native Windows-level file locking ensures only one instance of a script runs at a time.

### 2. Strict Memory Management (Arena Isolation)
- **Zero-Overhead Memory**: Uses a specialized Arena Allocator for instantaneous memory reclamation.
- **Glass Box Sandboxing**: All logic inside `try ... end` blocks is fully isolated. Memory is cleared immediately upon block exit.
- **Pointer Restriction**: Strict bounds-checking on all pointer dereferences (`*` and `@`) to prevent Segmentation Fault exploits.

### 3. Professional Toolchain
- **Opaque ABI**: Library generation (`.siz.lib`) features automatic symbol stripping and obfuscation (Internal names are renamed to random codes like `f1`, `s5`).
- **High-Precision Errors**: Visual, caret-based error reporting for rapid debugging.
- **VS Code Support**: Premium syntax highlighting (C++ Style) for `.six`, `.siz`, and `.sixlib` files.

## 🛠️ Getting Started

### Prerequisites
- **Nix** (Recommended for reproducible environment)
- **Rust** (Core Toolchain)
- **Zig** (Native Engine - Optional Fallback to C available)

### Build & Run
```powershell
# Build a .six script into an encrypted .siz binary
.\six.bat build test.six

# Run the encrypted binary
.\six.bat run release/test.siz

# Generate a secure, obfuscated library
.\six.bat lib crypto.six
```

## 📜 Example Syntax
```six
six MyProject
    fn main()
        try
            v x: i32 = 500
            v p = &x
            *p = 999999
            put *p
        end
    end
end
```

## 📦 VS Code Extension
The extension is located in `/vscode-extension`. To install:
1. Open VS Code.
2. Select **Install from VSIX...**
3. Choose `six-lang-1.0.1.vsix`.

---
**Developed with ❤️ for high-security systems.**
