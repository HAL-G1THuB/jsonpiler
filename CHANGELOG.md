# Project History and Plans

## Planned

- Implement `if` function
- Implement bitwise operations
- Support assignment of `bool` type

## Released

### jsonpiler 0.2

#### 0.2.3

- Refactored and cleaned up source code.
- Duplicate object keys are no longer allowed.
- Added temporary value tracking to enable efficient stack freeing, minimizing stack waste except for variable bindings.
- Improved error formatting to display ^ markers spanning the full error range using `pos.size`.

#### 0.2.2

- Added `examples` directory.
- Built-in functions now have priority over user-defined functions.
- To give priority to user-defined functions, write them as `["$", "function"]`.
- Added the ability to efficiently store free space on the stack and fill that space as needed.
- Unary `-` now correctly negates a single argument.

#### 0.2.1

- Local variables are now stored on the stack.
- Fixed a problem that caused the assembler to fail if `+`, `-`, or `*` takes more than 32-bit literals as operands.
- Added the ability to allocate stacks at the start of a scope and release them at the end.

#### 0.2.0

- `begin` no longer introduces a scope; instead, the new `scope` function handles scoping.
- Added new functions: `scope`, `*`, `list`, and `'`.

### jsonpiler 0.1

#### 0.1.11

- Added caching and reuse of string literals.
- Generalized variable generation in assembly code.
- Significantly renamed internal structures for improved clarity.

#### 0.1.10

- Greatly improved parser performance by addressing regressions from the previous version.

#### 0.1.9

- Added functionality to dynamically change stack allocation (currently unused).
- Fixed the error message formatting.
- Reduced parser overhead.

#### 0.1.8

- Added the new function `global`.
- Changed the structure of the variable table.
- `begin` now introduces a new scope.

#### 0.1.7

- The main function now returns `ExitCode` instead of `!`.
- Implemented temporary register storage with automatic save and restore.
- Internal functions are now included in the binary only when needed.

#### 0.1.6

- The `=` function now returns `null`.
- Fixed a documentation build error on docs.rs.
- Enhanced `$` operator to allow assignment to more types of values.

#### 0.1.5

- Added `CHANGELOG.md` to track project updates.

#### 0.1.4

- Fixed a typo in the previous crate.

#### 0.1.3

- Object entries now preserve insertion order.
- Object value evaluation now follows insertion order.

#### 0.1.2

- Added a Mermaid diagram to `README.md`.
- Fixed a bug affecting the evaluation order of expressions.

#### 0.1.0 ~ 0.1.1

- Transitioned from the previous crate.
