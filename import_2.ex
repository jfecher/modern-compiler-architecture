import import_2_1
import import_2_2

// import_2 also defines add10_conflicting, and input.ex imports both versions
def add10_conflicting: Int -> Int = fn x ->
    sub3 x + 13

def unused_in_import2 = 31
