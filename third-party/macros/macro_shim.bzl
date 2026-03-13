load("@prelude//rust:cargo_package.bzl", "cargo")
load("@prelude//decls:rust_rules.bzl", "rust_proc_macro_propagation")

# Shim for rust_library that correctly handles proc-macro crates.
# See https://buck2.build/docs/rfcs/drafts/plugin-deps/#example-proc-macros
def _rust_library(**kwargs):
    if kwargs.get("proc_macro") == True:
        name = kwargs["name"]
        real = name + "_REAL"
        rust_proc_macro_propagation(name = name, actual = ":" + real)
        kwargs["name"] = real

    cargo.rust_library(**kwargs)

shim = struct(
    rust_library = _rust_library,
)
