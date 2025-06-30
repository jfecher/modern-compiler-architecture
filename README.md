# Status

**Working**(*) - could use more explanation in places.

(*) We don't check generalized types for escaped type variables!
See [algorithm-j](https://github.com/jfecher/algorithm-j) for a more complete example of type inference.

# Intro

This compiler is a learning resource meant to show one possible architecture of a modern compiler
that is incremental, concurrent, and fault-tolerant. It takes into account the recent
move away from a traditional pipeline-style compiler towards a demand-driven compiler
designed for faster rebuilds with a language server in mind.

This is not at all meant to be a maximally performant or featureful compiler.
Instead, I'm prioritizing code clarity and size to better convey to those getting into
compiler development how one may go about writing such a compiler.

This codebase uses Rust as the implementation language and [inc-complete](https://github.com/jfecher/inc-complete) as the library
for incremental computations due to its support for serialization but the general techniques
should be applicable to any language. If you have any questions on techniques used or just
trouble understanding parts of the codebase please feel free to open an issue!

# Compiler features

- Incremental
  - Generally speaking, after the first successful compilation, the only parts of the program
  that are rebuilt are those that have changed or those that have used parts that have changed.
  For example, if we only add a `println` to the top level of our program, we should expect
  any libraries used not to be re-parsed, re-nameresolved, re-typechecked, etc.
  - In real compilers there is a tradeoff on how fine-grained or course-grained your incremental
  caching is. If you are too fine-grained you're caching a lot and may be slowed down performing
  too many equality checks and lugging around extra storage, but if you are too course grained you
  may be unnecessarily recompiling portions of the program which do not actually need to be recompiled.
  - This compiler generally caches each top-level item. More detail is given in `src/incremental.rs` for
  specific passes.
- Concurrent
  - When it is able to, the compiler prefers to perform work concurrently or in parallel, making
  use of more CPU cores.
  - The compiler uses concurrency in two places:
    1. When collecting and parsing source files to find changed files.
    2. After we collect a list of all source files used, we compile them all in parallel
- Fault-tolerant
  - Parser always produces a valid AST. If there are errors parsing it also returns a list of
  errors along with the AST. For example, if there is a syntax error in a top-level definition,
  the parser will recover by emitting an error and skipping tokens until the next top-level definition
  starts. In practice, recoverable parsers are essential if you are writing a language server
  where syntax errors are very common mid-edit.
  - Name resolution and type checking can continue on error and make some attempt not to emit
  duplicate errors (not perfect).
    - For example, a special type `Type::Error` is used for names which have failed to resolve. This
    type unifies with everything so we avoid issuing type errors for names which have already failed to resolve.

For more details on each, read the source files for each pass! They are commented and meant to be read.
As a good place to start, `src/main.rs` contains the entry point of the program where we (de)serialize
the compiler and run it on each input file. `src/incremental.rs` contains setup for `inc-complete` and
each function we cache. Individual passes are located in their own folder under `src`.

# Running the compiler

Make sure you have Rust installed. Afterward, just run `cargo run` in this directory after cloning
the repository and the compiler will compile `input.ex` and each file imported from it. You should
expect to see output which shows each query the compiler performs, and which thread it is done on:

```
Passes Run:
ThreadId(21):   - Collecting imports of input.ex
ThreadId(21):     - Parsing input.ex
ThreadId(19):   - Collecting imports of import_1.ex
ThreadId(04):   - Collecting imports of import_2.ex
ThreadId(19):     - Parsing import_1.ex
ThreadId(04):     - Parsing import_2.ex
ThreadId(04):   - Collecting imports of import_1_1.ex
ThreadId(04):     - Parsing import_1_1.ex
... etc
ThreadId(42):   - Compiling import_2_2.ex
ThreadId(41):   - Compiling import_2_1.ex
ThreadId(36):     - Collecting visible definitions in input.ex
ThreadId(44):       - Collecting exported definitions in import_1_2.ex
ThreadId(45):     - Collecting visible definitions in import_1_1.ex
ThreadId(36):       - Collecting exported definitions in input.ex
... etc
ThreadId(36):     - Type checking def add = fn x -> fn y -> + x y
ThreadId(38):     - Type checking def add10_conflicting: Int -> Int = fn x -> + (sub3 x) 13
ThreadId(36):       - Resolving def add = fn x -> fn y -> + x y
ThreadId(38):       - Resolving def add10_conflicting: Int -> Int = fn x -> + (sub3 x) 13
ThreadId(43):     - Type checking import import_1_2.ex
ThreadId(38):       - Get type of def sub3: Int -> Int = fn x -> + x 3
.. etc
Compiler finished.

errors:
  import_2_1.ex:4: Expected `=` but found `bar`
  input.ex:7: This imports `add10_conflicting`, which has already been defined here: import_1.ex:5
  input.ex:25: `never_defined` is not defined, was it a typo?
  input.ex:33: `defined_in_import_of_import` is not defined, was it a typo?
```

After that, try changing any of the source files to observe which computations are re-done!

# The language

The language was designed to be as simple as possible while also providing good points for
concurrent branching in the compiler.

Here's an example showing all the syntax in the language:

```boo
// Comments start with `//` and only line comments are supported

// We can declare and import external submodules with `import <name>`.
// This will look for files named `foo.ex` and `bar.ex` in the same directory.
// All definitions are imported automatically and all definitions are public
import foo
import bar

// Functions start with `def` and return an expression
// Type inference is supported
// You can think of this as `def add(x, y): return x + y` in python
def add = fn x y ->
    x + y

// Explicit types on a `def` can be specified:
def add2: Int -> Int -> Int =
    // Note that functions are curried automatically
    fn x y -> x + y

// Higher-order functions are supported
def apply = fn f x -> f x

// `print` is a top-level statement which outputs the result of an expression.
// These are executed from top to bottom. `print`s in a submodule are executed
// when the `import <name>` statement is used. In this program, we would print
// any `print`s in module `foo`, then `bar`, then this print.
print add 1 2
```

Note that the following features are _not_ supported:
- Any data type other than (a 64-bit) `Int` or functions
- Any operator other than `+` or `-`
- Mutual recursion in type inference
- Cycles in module imports (modules must form a directed acyclic graph)
