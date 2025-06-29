import import_1_1
import import_1_2

// import_2 also defines add10_conflicting, and input.ex imports both versions
def add10_conflicting = fn x ->
    add3 x + 7

def unused_in_import1 = 11
