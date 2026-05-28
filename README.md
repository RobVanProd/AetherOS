# Aero Programming Language

[![License: MIT](httpsa://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
<!-- Add other badges as appropriate, e.g., build status, version -->

## Overview

Aero is a modern, statically-typed programming language designed for performance, safety, and developer productivity. It aims to provide the control and speed of systems programming languages while offering high-level abstractions and a user-friendly ergonomic syntax.

**Core Goals:**
-   **Performance:** Achieve performance comparable to C/C++ through efficient compilation and zero-cost abstractions.
-   **Memory Safety:** Guarantee memory safety at compile time without a garbage collector, primarily through its ownership and borrowing system.
-   **Ergonomics:** Offer a clean, intuitive syntax and powerful tooling to enhance the developer experience.
-   **Concurrency:** Provide robust and safe concurrency features.

## Key Features

### Currently Implemented ✅

-   **Static Typing:** A strong, static type system with compile-time type checking and automatic type inference
-   **Type Inference:** Automatic type deduction for variables and expressions while maintaining type safety
-   **Memory Safety:** Stack-allocated variables with proper lifetime management
-   **Arithmetic Operations:** Full support for integer and floating-point arithmetic with automatic type promotion
-   **Variable System:** Immutable variables by default with explicit mutability support (`let` vs `let mut`)
-   **Comprehensive Error Reporting:** Clear error messages with type information and validation
-   **LLVM Backend:** Native code generation through LLVM for optimal performance
-   **CLI Tooling:** Complete command-line interface with `aero build` and `aero run` commands

### Phase 3 Features (Recently Implemented) ✅

-   **Function Definitions:** Complete support for defining and calling functions with parameters and return types (`fn name(params) -> type`)
-   **Control Flow:** Full implementation of if/else statements, while loops, for loops, and infinite loops with break/continue
-   **I/O Operations:** Working print macros (`print!`, `println!`) with format string validation and printf integration
-   **Enhanced Type System:** All comparison operators (`==`, `!=`, `<`, `>`, `<=`, `>=`) and logical operators (`&&`, `||`, `!`) implemented
-   **Advanced Scoping:** Complete nested scopes, variable shadowing, and function-local variables
-   **Variable Mutability:** Full support for mutable (`let mut`) and immutable (`let`) variables with compile-time enforcement
-   **Semantic Validation:** Comprehensive compile-time checking for all language constructs with detailed error reporting
-   **LLVM Code Generation:** Full LLVM IR generation for functions, control flow, I/O operations, and enhanced expressions
-   **Enhanced Error Reporting:** Source location tracking with line/column information and helpful error suggestions

### Planned Features 📋

-   **Ownership and Borrowing:** A compile-time memory management system that prevents dangling pointers, data races, and other common memory-related bugs without a garbage collector
-   **Generics and Traits:** Powerful tools for creating reusable abstractions and achieving polymorphism
-   **Pattern Matching:** Expressive control flow and destructuring of data
-   **Data Structures:** Structs, enums, arrays, and other composite types
-   **Modular Design:** Code organization into modules for better structure and reusability
-   **Comprehensive Standard Library:** Essential utilities for common programming tasks

## Current Status

**Version:** 0.3.0 (Phase 3 Complete)  
**Status:** Core Language Features Implemented

Aero has successfully completed Phase 3 development, transforming from a basic arithmetic calculator into a functional programming language with modern features. The compiler now supports:

- ✅ Complete function definition and call system
- ✅ Full control flow constructs (if/else, loops, break/continue)
- ✅ I/O operations with format string support
- ✅ Enhanced type system with all operators
- ✅ Advanced variable scoping and mutability
- ✅ Comprehensive error reporting and validation
- ✅ LLVM code generation for all features

## Quick Start

### Installation

1. Clone the repository:
```bash
git clone https://github.com/aero-lang/aero.git
cd aero
```

2. Build the compiler:
```bash
cd src/compiler
cargo build --release
```

3. Add to PATH (optional):
```bash
# Add the target/release directory to your PATH
export PATH=$PATH:$(pwd)/target/release
```

### Your First Aero Program

Create a file called `hello.aero`:

```aero
fn greet(name: &str) -> () {
    println!("Hello, {}!", name);
}

fn main() {
    let name = "World";
    greet(name);
    
    // Demonstrate control flow
    let mut count = 0;
    while count < 5 {
        if count % 2 == 0 {
            println!("{} is even", count);
        } else {
            println!("{} is odd", count);
        }
        count = count + 1;
    }
}
```

Compile and run:
```bash
aero run hello.aero
```

Or compile to LLVM IR:
```bash
aero build hello.aero -o hello.ll
```

## Language Examples

### Function Definitions
```aero
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

fn factorial(n: i32) -> i32 {
    if n <= 1 {
        return 1;
    }
    return n * factorial(n - 1);
}
```

### Control Flow
```aero
fn main() {
    // If/else statements
    let x = 10;
    if x > 5 {
        println!("x is greater than 5");
    } else {
        println!("x is 5 or less");
    }
    
    // While loops
    let mut i = 0;
    while i < 3 {
        println!("Iteration: {}", i);
        i = i + 1;
    }
    
    // For loops
    for j in 0..5 {
        if j == 3 {
            break;
        }
        println!("For loop: {}", j);
    }
}
```

### I/O Operations
```aero
fn main() {
    let name = "Aero";
    let version = 3;
    
    print!("Welcome to ");
    println!("{} v0.{}!", name, version);
    
    let a = 15;
    let b = 4;
    println!("{} + {} = {}", a, b, a + b);
    println!("{} * {} = {}", a, b, a * b);
}
```

## Project Structure

```
aero/
├── src/compiler/          # Main compiler source code
│   ├── src/
│   │   ├── lexer.rs      # Tokenization and lexical analysis
│   │   ├── parser.rs     # Syntax analysis and AST generation
│   │   ├── semantic_analyzer.rs  # Type checking and validation
│   │   ├── ir_generator.rs       # Intermediate representation
│   │   ├── code_generator.rs     # LLVM code generation
│   │   └── main.rs       # CLI interface
│   └── Cargo.toml        # Rust project configuration
├── examples/             # Example Aero programs
│   ├── calculator.aero   # Complex calculator demo
│   ├── fibonacci.aero    # Recursive fibonacci
│   ├── loops.aero        # Control flow examples
│   └── scoping.aero      # Variable scoping demo
├── benchmarks/           # Performance benchmarks
├── tutorials/            # Learning materials
└── README.md            # This file
```

## Development Roadmap

### Phase 4: Data Structures (Next)
- Arrays and slices
- Structs and methods
- Enums and pattern matching
- Tuple types

### Phase 5: Advanced Features
- Ownership and borrowing system
- Generics and traits
- Module system and imports
- Error handling with Result types

### Phase 6: Standard Library
- Collections (Vec, HashMap, etc.)
- String manipulation
- File I/O and networking
- Concurrency primitives

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details on:

- Setting up the development environment
- Code style and conventions
- Testing requirements
- Pull request process

### Development Setup

1. Install Rust (1.70+ required)
2. Install LLVM development libraries
3. Clone and build:
```bash
git clone https://github.com/aero-lang/aero.git
cd aero/src/compiler
cargo build
cargo test
```

## Testing

Run the test suite:
```bash
cd src/compiler
cargo test                    # Unit tests
cargo test --test integration # Integration tests
cargo bench                   # Performance benchmarks
```

## Performance

Verified benchmark artifacts are kept under
[`claim-verification/`](claim-verification/). The latest local verification run
was captured on 2026-05-28 at commit
`f263568e06073317467416c2f954c73927c00f3e` on a machine where ROCm reported a
Radeon RX 7900 XTX (`gfx1100`, PCI device `1002:744c`) and the integrated AMD
Radeon Graphics device (`gfx1036`, PCI device `1002:13c0`).

Current verified results from that run:

- Python benchmark harness compilation means ranged from 0.0385s to 0.0474s
  across the checked benchmark inputs
  ([raw JSON](benchmarks/results/performance_results_1780006109.json)).
- Rust Criterion lexer benchmarks measured tokenization cases from 340.38ns to
  22.956us
  ([raw log](claim-verification/results/aero_post_reboot_7900xtx_20260528T190000Z/run_performance_benchmarks.stdout.log)).

The verification run did not produce evidence for GPT-2 training throughput,
GPU matmul speedups, NCCL/MPI scaling, GGUF inference benchmarks, or HIP
vector-add benchmarks. Those claims are omitted from this README until raw
artifacts or fresh reruns support them.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- LLVM Project for the backend infrastructure
- Rust community for inspiration and tooling
- All contributors and early adopters

---

**Status:** Active Development | **Version:** 0.3.0 | **License:** MIT

Aero is an actively developed programming language. It is currently in **Phase 3: Core Language Features** (as outlined in our [Roadmap.md](Roadmap.md)). Phase 2 has been completed successfully, and we are now implementing advanced language features including functions, control flow, and I/O operations.

**Phase 2 Complete ✅** - The compiler now has:
- Full lexical analysis and parsing
- Complete semantic analysis with type checking
- LLVM IR generation and native compilation
- Working CLI tools (`aero build` and `aero run`)
- Comprehensive test suite and CI/CD

**Phase 3 Nearly Complete ✅** - Successfully implemented:
- ✅ Function definitions and calls with full semantic validation
- ✅ Control flow statements (if/else, loops) with break/continue support
- ✅ I/O operations (print!, println!) with printf integration
- ✅ Enhanced type system with all comparison and logical operators
- ✅ Advanced scope management with variable mutability and shadowing
- ✅ Complete LLVM code generation for all new features
- 🚧 Enhanced error reporting with source locations (remaining work)

The language is still under active development and the benchmark claims above
are limited to the artifacts cited there. We welcome feedback and contributions.

## Getting Started

Ready to try Aero? Here’s how to get started:

### Installation

To use Aero, you'll first need to install its compiler, `aero`. The compiler is written in Rust, so you'll need Rust and Cargo installed. You'll also need LLVM tools (`llc` and `clang`) for compilation.

#### Prerequisites

1. **Install Rust and Cargo:**
   Visit [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install) and follow the installation instructions.

2. **Install LLVM tools:**
   - **Ubuntu/Debian:** `sudo apt install llvm clang`
   - **macOS:** `brew install llvm` (or use Xcode command line tools)
   - **Windows:** Download from [LLVM releases](https://releases.llvm.org/) or use `winget install LLVM.LLVM`

#### Installation Steps

1.  **Clone the Aero Repository:**
    ```bash
    git clone https://github.com/RobVanProd/Aero.git
    cd Aero
    ```

2.  **Install the Aero Compiler:**
    From the root directory of the Aero project, run:
    ```bash
    cargo install --path src/compiler
    ```
    This command builds the `aero` binary from the source code in `src/compiler/` and installs it into your Cargo binary directory (usually `~/.cargo/bin/`).

3.  **Verify Installation:**
    Ensure `~/.cargo/bin` is in your system's `PATH`. Then, open a new terminal session and type:
    ```bash
    aero --help
    ```
    This should display the Aero compiler help message.

#### Testing the Installation

To verify that everything is working correctly, you can run the test suite:

**Linux/macOS:**
```bash
chmod +x test_compiler.sh
./test_compiler.sh
```

**Windows:**
```cmd
test_compiler.bat
```

The test suite will build the compiler and run several example programs to ensure everything is working correctly.

### Your First Aero Program

Let's create a simple program that demonstrates Aero's current capabilities.

1.  Create a file named `example.aero` with the following content:
    ```aero
    // example.aero - Demonstrates Phase 3 Aero features
    
    fn greet(name: &str) {
        println!("Hello, {}!", name);
    }
    
    fn calculate_factorial(n: i32) -> i32 {
        if n <= 1 {
            return 1;
        } else {
            return n * calculate_factorial(n - 1);
        }
    }
    
    fn main() {
        println!("=== Aero Language Demo ===");
        
        // Variables and mutability
        let x = 10;
        let mut y = 5;
        y = y + 3;
        
        println!("x = {}, y = {}", x, y);
        
        // Function calls
        greet("Aero Developer");
        
        // Control flow and recursion
        let fact = calculate_factorial(5);
        println!("5! = {}", fact);
        
        // Loops
        let mut i = 0;
        while i < 3 {
            print!("Count: {} ", i);
            i = i + 1;
        }
        println!("");
        
        println!("Demo complete!");
    }
    ```

This program showcases:
- Function definitions with parameters and return types
- Variable declarations with mutability control
- I/O operations with formatted printing
- Control flow (if/else statements)
- Loops (while loops)
- Recursion and function calls

### What's Currently Working

Aero can currently compile and run programs with these features:

**Variables and Mutability:**
```aero
let x = 42;           // Immutable integer variable
let mut y = 3.14;     // Mutable float variable
y = y + 1.0;          // Reassignment to mutable variable
let z = x + y;        // Mixed arithmetic with type promotion
```

**Functions:**
```aero
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

fn greet(name: &str) {
    println!("Hello, {}!", name);
}

fn main() {
    let result = add(5, 3);
    greet("World");
}
```

**Control Flow:**
```aero
// If/else statements
if x > 0 {
    println!("Positive");
} else if x < 0 {
    println!("Negative");
} else {
    println!("Zero");
}

// While loops
let mut i = 0;
while i < 5 {
    println!("Count: {}", i);
    i = i + 1;
}

// Infinite loops with break/continue
loop {
    if condition {
        break;
    }
    continue;
}
```

**I/O Operations:**
```aero
print!("Hello ");           // Print without newline
println!("World!");         // Print with newline
println!("Value: {}", 42);  // Formatted printing
println!("Sum: {}", a + b); // Expression formatting
```

**Enhanced Type System:**
```aero
// Comparison operators
let equal = (a == b);
let not_equal = (a != b);
let less = (a < b);
let greater = (a > b);

// Logical operators
let and_result = (true && false);
let or_result = (true || false);
let not_result = !true;
```

### Compiling and Running

There are two main ways to compile and run your Aero programs:

1.  **Build then Run:**
    First, compile your program into an executable:
    ```bash
    aero build example.aero -o example_executable
    ```
    This creates an executable file named `example_executable`. You can then run it:
    -   On Linux/macOS: `./example_executable`
    -   On Windows: `.\example_executable.exe`

2.  **Compile and Run Directly:**
    For convenience, you can compile and immediately run your program with a single command:
    ```bash
    aero run example.aero
    ```

The program will execute and return an exit code of 13 (the result of 10 + 3.5, truncated to integer). You can check the exit code with:
- Linux/macOS: `echo $?`
- Windows: `echo %ERRORLEVEL%`

## Learning Aero

Dive deeper into the Aero language with these resources:

-   **Tutorials**:
    -   [Tutorial 1: Getting Started](tutorials/01-getting-started.md)
    -   [Tutorial 2: Core Language Features](tutorials/02-core-features.md)
    -   [Tutorial 3: Ownership and Borrowing](tutorials/03-ownership-borrowing.md)
    -   [Tutorial 4: Data Structures (Structs, Enums, etc.)](tutorials/04-data-structures.md)
-   **Example Programs**:
    -   [fibonacci.aero](examples/fibonacci.aero) - Functions and recursion
    -   [loops.aero](examples/loops.aero) - All loop types and control flow
    -   [calculator.aero](examples/calculator.aero) - I/O operations and functions
    -   [scoping.aero](examples/scoping.aero) - Variable scoping and mutability
    -   [error_examples.aero](examples/error_examples.aero) - Common error cases
-   **Language Design Documents**:
    -   [Aero Grammar (EBNF)](aero_grammar.md)
    -   [Type System Rules](aero_type_system.md)
    -   [Ownership and Borrowing Model](aero_ownership_borrowing.md)
-   **Troubleshooting**:
    -   [Troubleshooting Guide](TROUBLESHOOTING.md) - Solutions to common issues

## Standard Library

Aero aims to provide a useful standard library to assist with common programming tasks. The design and features of the standard library are currently being defined.

-   Learn more about the proposed standard library structure and APIs in the [Standard Library RFC](RFCs/standard-library.md).

## Contributing

Aero is an open-source project, and we welcome contributions from the community! Whether you're interested in language design, compiler development, writing documentation, or creating examples, there are many ways to help.

-   **Contribution Guidelines**: Please read [CONTRIBUTING.md](CONTRIBUTING.md) (if it exists, otherwise check for community guidelines or open an issue to ask).
-   **Code of Conduct**: We adhere to the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). Please ensure you read and follow it.
-   **RFCs (Request for Comments)**: Major language design changes and new features are discussed through an RFC process. Check out the [RFCs directory](RFCs/) to participate or propose new ideas.
-   **Issues**: Report bugs or suggest features by opening an issue on our GitHub repository.
-   **Pull Requests**: We welcome well-tested pull requests for bug fixes, feature implementations, and documentation improvements.

## Roadmap

To see the current development phase and future plans for Aero, please refer to the [Roadmap.md](Roadmap.md).

## License

Aero is distributed under the terms of the MIT license.

See [LICENSE](LICENSE) for details.
