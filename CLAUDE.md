# Project: `dioxus_codemirror`

`dioxus` component that wraps CodeMirror 6.


## Code Style / Patterns

1. Variable names should usually be the snake case version of their type, e.g. for the `NodeId`, use `node_id`.
2. When working with multiple variables of the same type, favour a common prefix so that they are grouped when sorted by name, e.g. `node_id_from`, `node_id_to`.
3. When creating functions, favour naming them in `noun_verb` order, so they are discoverable when searching by name, and grouped together when sorted. e.g. `thing_rename`, `thing_delete`.
4. When creating functions, favour adding them within a type's `impl` block instead of a non-scoped function, This helps group related functions together. e.g. `impl ThingBuilder { fn node_coordinates(thing: &Thing) -> Point<f32, f32> { .. } }`.
5. When extracting functions to keep function bodies small, if it is in the same type, the calling function name should be a prefix of the called function name, so that it is easy to identify functions that are easy to extract. If the extracted function is used by multiple calling functions, often that indicates it should be extracted to a separate type.
6. Favour using strong types, e.g. `NodeId` when handling Node IDs, but `Id` when handling different kinds of IDs (e.g. `NodeId`, `EdgeId`, etc.), instead of using `String` / `&str`, so that it is clear what concept the variable should represent, and the constraints built into those types are guaranteed.
7. This means do NOT map a type into a `String` / `&str` unless necessary to compile.
8. If you must use "stringly typed" / standard data types (e.g. primitive types, `String`, `Map`, `Vec`, `Set`, etc.), then instead of using tuples, create a new type with field names that indicate what information those types hold. e.g. instead of `(String, Vec<String>)`, use `struct NodeIdToCssClasses { node_id: String, css_classes: Vec<String>, }`
9. Each type usually is defined in its own snake_case module, e.g. `NodeId` would be in `node_id.rs`.
10. "Data model" types are usually separate from "logic" types -- so data models can be published without publishing logic. Sometimes logic modules contain supporting data types to pass parameters.
11. Try and keep functions under 200 lines by extracting logic / components with meaningful names.
12. Avoid non-ascii characters, e.g. "—". Use "--" for elaboration or "`param`: description" in parameter documentation.
13. Section comments should be written as `// === Section Name === //` instead of a 3 line comment.
14. Documentation should include example valid values.
15. Unless a type / component is small, it should be placed in its own module -- often a submodule of the current module.
16. A module and its submodules would be `foo.rs` and `foo/bar.rs`, not `foo/mod.rs` and `foo/bar.rs`.
