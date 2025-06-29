// Comments start with `//` and only line comments are supported

// We can declare and import external submodules with `import <name>`. This will look for
// files named `import1.ex` and `import2.ex` in the same directory.
// All definitions are imported automatically and all definitions are public
import import_1
import import_2

// Functions start with `def` and return an expression
// Type inference is supported
// You can think of this as `def add(x, y): return x + y` in python
def add = fn x y ->
    // The only supported operators are `+` and `-`!
    x + y

// Explicit types on a `def` can be specified:
def add2: Int -> Int -> Int =
    // Note that functions are curried automatically
    fn x y -> x + y + 1

// Higher-order functions are supported
def apply = fn f x -> f x
def twice = fn f x -> f (f x)

def bad = never_defined

def add20 =
    // error: add10_conflicting imported from both import_1 and import_2
    twice add10_conflicting

def try_use_import_of_import =
    // expect this to error, this is defined in import_1_1 which is not imported here
    defined_in_import_of_import

// `print` is a top-level statement which outputs the result of an expression.
// These are executed from top to bottom. `print`s in a submodule are executed
// when the `import <name>` statement is used. In this program, we would print
// any `print`s in module `import_1`, then `import_2`, then this print.
print add 1 2
print add20 5
