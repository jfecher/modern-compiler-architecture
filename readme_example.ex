// Comments start with `//` and only line comments are supported

// We can declare and import external submodules with `import <name>`. This will look for
// files named `foo.ex` and `bar.ex` in the same directory.
// All definitions are imported automatically and all definitions are public
import foo
import bar

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
def twice = fn f x -> f (f x)

def bad = never_defined

def add20 = twice add10 //add10 imported from foo.ex

// `print` is a top-level statement which outputs the result of an expression.
// These are executed from top to bottom. `print`s in a submodule are executed
// when the `import <name>` statement is used. In this program, we would print
// any `print`s in module `foo`, then `bar`, then this print.
print add 1 2
print add20 5
