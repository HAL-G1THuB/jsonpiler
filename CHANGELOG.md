# Project History and Plans

## Planned

- Enables data to be stored in the heap area
- Allow heap release when exiting scope
- Eliminate assignment ambiguity and build ownership systems

## Released

### jsonpiler 0.4

#### 0.4.1

- Limiting input files to 1 GB or less eliminates unnecessary safety checks in the parser and speeds up the process.
- Changed the way assembly instructions are stored, improving processing speed and memory efficiency.
- Eliminated dependence on c functions (malloc, free), making `ucrtbase.dll` unnecessary.
- Split the documentation of built-in functions into several files because they became bloated.
- The timing of releasing a temporary value passed as an argument of a function not bound to a variable is now fixed at the end of the function. (Exception: the last temporary value of the body of `if` is released.)
- The argument format of `if`, `scope`, and `lambda` has been changed.
- A new function `value` has been added. This function returns the given evaluated value as-is and is used to add a literal to the end of an Object's instruction sequence.

#### 0.4.0

- Added new function: `not`, `xor`, `or`, and `and`.
- The built-in functions have become bloated and have been split into multiple files.
- Changed bool type memory area from 1bit to 1byte due to expected large performance degradation due to instruction bloat
- Intuitive function argument validation.

### jsonpiler 0.3

#### 0.3.5

- Added new function: `if`
- Fixed an issue in version 0.1.6 where docs.rs documents were sometimes not generated, and removed alternative documents that were no longer needed.
- Change space characters around instructions in the generated assembly to tab characters.
- Removed documentation comments with little content.
- Added Japanese version to changelog and README.md.

#### 0.3.4

- Support for `bool` type assignments.
- Refactor the code to generate assembly instructions and labels in a more maintainable and systematic way.

#### 0.3.3

- Fixed a bug where memory was not correctly released from the stack when handling scope-based logic.
- Fixed a situation where the `scope` function ignored the first argument.
- Removed redundant safety checks

#### 0.3.2

- `Float` can now be assigned.
- Function definitions now explicitly share only the global scope.
- Fixed a bug in the `global` function.

#### 0.3.1

- Added new functions: `/`, `abs`, and `%`.

#### 0.3.0

- Json objects now allow duplicate keys.
- **Objects are now treated as function calls**:  
  Each key in a JSON object is interpreted as a function name, and its corresponding value is treated as the function argument.
- It is no longer allowed to assign a user-defined function to a variable name that already exists as a built-in function.
- **Arrays now leave the result of evaluating all elements.**:  
- **Supports multiple key-function entries.**:  
  When an object contains multiple keys, each is evaluated in order; the last function result is returned.
- Square brackets can now be omitted when a single argument is not an array.
- The `begin` function was removed because it can now be represented by a column of objects.

### jsonpiler 0.2

#### 0.2.3

- Refactored and cleaned up source code.
- ~~Duplicate object keys are no longer allowed.~~
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

- Added functionality to dynamically change stack allocation ~~(currently unused)~~.
- Fixed the error message formatting.
- ~~Reduced parser overhead.~~

#### 0.1.8

- Added the new function `global`.
- Changed the structure of the variable table.
- ~~`begin` now introduces a new scope.~~

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
