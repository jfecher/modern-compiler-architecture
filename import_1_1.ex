// This is going to be imported in import_1.ex (but not re-exported from there)
// and referenced in input.ex where it should error because imports of imports
// should not be visible
def defined_in_import_of_import = 2
