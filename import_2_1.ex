// uh-oh syntax error! Make sure the parser can still
// recover and at least pick up the definition for `sub3` below!
def foo bar baz

def sub3: Int -> Int =
    fn x -> x - 3
