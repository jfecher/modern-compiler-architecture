# Status

**WIP - Please Ignore for Now!**

# Intro

This compiler is an example meant to show one possible architecture of a modern compiler
that is incremental, concurrent, and fault-tolerant. It takes into account the recent
move away from a traditional pipeline-style compiler towards a demand-driven compiler
designed for faster rebuilds with a language server in mind.

This is not at all meant to be a maximally performant or featureful compiler.
Instead, I'm prioritizing code clarity and size to better convey to those getting into
compiler development how one may go about writing such a compiler.

This codebase uses Rust as the implementation language and inc-complete as the library
for incremental computations but the general techniques should be applicable to any language.
If you have any questions on techniques used or just trouble understanding parts of the
codebase please feel free to open an issue!

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
  - This compiler generally caches each top-level item. More detail is given in the source code for
  specific passes.
- Concurrent
  - When it is able to, the compiler prefers to perform work concurrently or in parallel, making
  use of more CPU cores.
- Fault-tolerant
  - Parser always produces a valid AST. If there are errors parsing it also returns a list of
  errors along with the AST. For example, if there is a syntax error in a top-level definition,
  the parser will recover by emitting an error and skipping tokens until the next top-level definition
  starts. In practice, recoverable parsers are essential if you are writing a language server
  where syntax errors are very common mid-edit.
  - Name resolution and type checking can continue on error and make some attempt not to emit
  duplicate errors (not perfect).

For more details on each, read the source files for each pass! They are commented and meant to be read.

# The language

The language was designed to be as simple as possible while also providing good points for
concurrent branching in the compiler.

Here's an example showing all the syntax in the language:

```groovy
// Comments start with `//` and only line comments are supported

// We can declare external submodules with `module <name>`. This will look for
// files named `foo.ex` and `bar.ex` in the same directory.
// All definitions are imported automatically and all definitions are public
module foo
module bar

// Functions start with `def` and return an expression
// Type inference is supported
// You can think of this as `def add(x, y): return x + y` in python
def add = fn x y ->
    // The only supported operators are `+` and `-`!
    x + y

// Explicit types on a `def` can be specified:
def add2: Int -> Int -> Int =
    // Note that functions are curried automatically
    fn x y -> x + y

// Higher-order functions are supported
def apply = fn f x -> f x

// `print` is a top-level statement which outputs the result of an expression.
// These are executed from top to bottom. `print`s in a submodule are executed
// when the `module <name>` statement is used. In this program, we would print
// any `print`s in module `foo`, then `bar`, then this print.
print add 1 2
```

Note that the following features are _not_ supported:
- Any data type other than (a 64-bit) `Int` or functions
- Mutual recursion in type inference
- Cycles in module declarations (modules must form a directed acyclic graph)
