# changelog

## Planned

- Implementation of variable-length arrays
- Implementation of structures
- Implementation of an ownership system or a garbage collector

## Released

### jsonpiler 0.9

#### 0.9.2

- Changed:
  - Version information is now automatically generated from `Cargo.toml`

#### 0.9.1

- Fixed
  - Issue where `!=` did not work correctly with `Str`

#### 0.9.0

- Added
  - Commands: `format`, `server`
  - New functions: `confirm`, `main`, `<<`, `>>`
  - Formatting feature and an LSP server for diagnostics and error reporting
  - Local variable definition with `let` (`variable = value`)
  - Global variable definition with `global` (`variable = value`)
  - Comparison functions now support `Float`
  - Operator precedence

- Changed
  - Syntax for variable definition and reassignment has been separated
  - `=` is now used exclusively for reassignment
  - In `if([cond, any])`, the `[]` can be omitted when only a single condition–value pair is present
  - `concat` now concatenates non-literal values as well
  - Unexpected memory leaks are now detected at runtime
    (memory leaks are not expected by design)
  - When loading other files with `include`, they are executed at startup rather than at the time of first load
  - Warnings are now emitted for unused variables and arguments
    This warning can be suppressed by prefixing the variable name with `_`
  - `len` now returns the number of characters in a string rather than the byte length
  - The function name invoked by `GUI` is now displayed in the window title bar
  - Only user-defined functions that are used, along with the functions they depend on, are linked
  - Now generates an error for arithmetic overflow in non-release builds
- Removed
  - `cargo doc`
  - Functions: `'`, `eval`

### jsonpiler 0.8

#### 0.8.0

- Added
  - Commands: `help`, `version`, ` `, `release`, `build`, `build release`
  - In `release` builds, debug information is removed from the `.exe`
  - Heap memory management system
  - New functions: `sqrt`, `input`, `Str`
  - Compound assignment operators: `+=`, `-=`, `*=`, `/=`
  - `error_cases` folder
  - Error location and related information in runtime errors

- Changed
  - `String` type → `Str` type
  - Generated `.exe` files now include Jsonpiler version information
  - A new `GUI` can now be created after the previous one exits
  - Correct mouse position retrieval even when the cursor is outside the `GUI` window
  - `GUI` no longer terminates forcibly when the PC enters sleep mode
  - ~~The filename containing `GUI` is displayed in its title bar~~
  - Error messages are now shown when Ctrl+C is pressed or when the system shuts down

### jsonpiler 0.7

#### 0.7.4

- Changed
  - Improvements to error checking and error message generation

#### 0.7.3

- Added
  - SEH (Structured Exception Handling)

- Changed
  - Compile error propagation structured using `Enum`
  - ~~Runtime division-by-zero errors delegated to SEH~~
  - Optimization when any of the second or later operands in arithmetic operations is `0` at compile time
  - Evaluating an empty object now returns `Null`

- Fixed
  - Bug in reassignment of `global`

- Removed
  - Limit on the number of arguments for user-defined functions

#### 0.7.2

- Added
  - Early return mechanism for functions: `ret`
  - Loop control mechanisms: `break`, `continue`

- Changed
  - Optimizations for integer storage and local memory access reduced about 30% of the machine code section in generated `.exe` files

#### 0.7.1

- Added
  - `Int` bitwise operations for `not`, `and`, `or`, `xor`

- Changed
  - Improved memory efficiency and structure of the assembly intermediate representation
  - Added constant folding optimization for `Int` arithmetic operators (`+`, `-`, `*`, `/`, `%`)
  - String literals are now placed in the `.rdata` section

#### 0.7.0

- Added
  - GUI feature: `GUI`
  - JSPL now supports a new syntax: `1 + 10 + 1` is interpreted the same as `+(1, 10, 1)`
  - Executables generated using the `GUI` function require `gdi32.dll` (included by default in 64-bit Windows)

- Changed
  - `global` is now reassignable
  - `global` is now thread-safe
  - In JSPL, semicolons are required when writing multiple function calls on the same line

### jsonpiler 0.6

#### 0.6.5

- Added
  - New functions: `assert`, `random`
  - Comparison functions: `>`, `>=`, `!=`
  - Module system: `include`
  - The `abs` function now supports the `Float` type

- Changed
  - The `Int` type can now correctly handle its minimum value (`0xffffffffffffffff`)
  - Error message format
  - The `print` function now supports pipes and redirection

- Fixed
  - An issue where the sign of a `Float` value was not correctly inverted when passed to the `-` function

#### 0.6.4

- Fixed
  - An issue where the fifth and subsequent arguments of user-defined functions were not recognized correctly

#### 0.6.3

- Added
  - Arithmetic operations `+`, `-`, `*`, `/` now support the `Float` type
  - Function to truncate a `Float` value to an integer: `Int`
  - I/O function: `print`

- Fixed
  - Error in the `global` function

#### 0.6.2

- Added
  - New function: `len`

- Changed
  - ~~Maximum number of arguments for user-defined functions set to 16~~
  - Jsonpiler now returns the correct executable exit code

- Fixed
  - An issue where boolean values were not correctly passed to user-defined functions
  - An issue where strings stored in local variables were not recognized correctly

#### 0.6.1

- Added
  - JSPL
    JSPL (Jsonpiler Structured Programming Language) is a new syntax introduced for Jsonpiler, designed to make programs easier for humans to write and read
  - Japanese version of the language specification

#### 0.6.0

- Added
  - Loop construct: `while`
  - Comparison functions: `==`, `<`, `<=`
  - `define`, which supports function registration and recursive calls

- Changed
  - Reassignment to local variables is now possible for most types

- Fixed
  - An issue where escape characters were not displayed correctly due to the integration of the built-in assembler

- Removed
  - `Function` type
  - Existing `lambda`

---

### jsonpiler 0.5

#### 0.5.3 ~ 0.5.4

- Fixed
  - Critical bug introduced in 0.5.0

#### 0.5.1 ~ 0.5.2

- Fixed
  - Issue where `*.bin binary` was not added to `.gitattributes`

#### 0.5.0

- Added
  - A custom assembler and linker integrated into jsonpiler

- Fixed
  - An issue where the assembler produced an error when values larger than 32 bits were used in certain `mov` instructions

- Removed
  - Generation of `.s` files
  - Generation of `.obj` files
  - Dependency on GNU `as`
  - Dependency on GNU `ld`
  - ~~SEH (temporarily)~~

### jsonpiler 0.4

#### 0.4.2

- Added
  - New function: `concat`
  - Split `Object` into three variants
    - **HashMap**: represents a collection of key–value pairs
    - ~~Sequence~~ **Block**: represents an ordered sequence of instructions
    - **TypeAnnotations**: represents type annotations for variables or functions
  - Arguments for `lambda`

- Changed
  - Expanded the range of types that can be returned by `lambda`
  - Arithmetic functions now produce an error when called without arguments
  - Minimum number of arguments for `+`, `/`, `*`, `or`, `and`, `xor` is now 2
  - The return value of `message` is now `Null`

#### 0.4.1

- Added
  - Function used to specify the return value of a `Block`: `value`

- Changed
  - Method of storing assembly instructions
  - Built-in function documentation was split into multiple files due to its size
  - The timing for freeing temporary values passed as arguments to functions (when they are not bound to variables) is now defined as when the function finishes execution
    (Exception: the final temporary value in the body of `if` is freed)
  - Argument formats for `if`, `scope`, and `lambda` were changed

- Removed
  - Dependency on C functions (`malloc`, `free`), making `ucrtbase.dll` unnecessary
  - Safety checks that became unnecessary after restricting input files to under 1GB

#### 0.4.0

- Added
  - Logical operation functions: `not`, `xor`, `or`, `and`

- Changed
  - Built-in functions were split into multiple files due to their increasing size
  - Because instruction expansion was expected to degrade performance, the memory size for the `bool` type was changed from 1 bit to 1 byte
  - Function argument validation was made more intuitive

### jsonpiler 0.3

#### 0.3.5

- Added
  - Japanese versions of CHANGELOG and README
  - `if` function

- Changed
  - ~~Changed whitespace around generated assembly instruction sequences to tabs~~

- Removed
  - Documentation comments with little substantive content
  - Alternative documentation that became unnecessary after the 0.1.6 documentation bug fix

#### 0.3.4

- Added
  - Support for assigning the `bool` type

- Changed
  - A more systematic system for generating assembly instructions and labels to improve maintainability

#### 0.3.3

- Fixed
  - A bug where memory was not correctly freed from the stack when processing scope-based logic
  - A bug where the `scope` function ignored its first argument

- Removed
  - Redundant safety checks

#### 0.3.2

- Added
  - Support for assigning the `Float` type

- Changed
  - Function definitions now explicitly share only the global scope

- Fixed
  - Bug in the `global` function

#### 0.3.1

- Added
  - New functions `/`, `abs`, `%`

#### 0.3.0

- Changed
  - JSON objects now allow duplicate keys
  - Objects are now treated as function calls:
    Each key in a JSON object is interpreted as a function name, and the corresponding value is treated as the function's argument
  - It is no longer possible to assign a user-defined function to a variable name that already exists as a built-in function
  - Arrays now preserve the evaluation results of all elements
  - Support for multiple key-function entries:
    If an object contains multiple keys, each is evaluated in order, and the result of the last function is returned
  - Square brackets can now be omitted when the argument is not an array

- Removed
  - `begin` function

### jsonpiler 0.2

#### 0.2.3

- Added
  - Tracking of temporary values to enable efficient stack deallocation

- Changed
  - ~~Duplicate object keys are no longer allowed~~
  - Improved error formatting to display `^` markers across the full error range using `pos.size`

#### 0.2.2

- Added
  - `examples` directory
  - Compile-time stack allocation feature

- Changed
  - ~~Built-in functions take precedence over user-defined functions~~
  - ~~To prioritize user-defined functions, write `["$", "function"]`~~
  - `-` negates the value when called with a single argument

#### 0.2.1

- Added
  - Stack allocation at the start of a scope and stack deallocation at the end

- Changed
  - Local variables are now stored on the stack

- Fixed
  - Fixed an issue where the assembler failed when `+`, `-`, or `*` received literals larger than 32 bits

#### 0.2.0

- Added
  - New functions: ~~`eval`~~, `list`, ~~`'`~~

- Changed
  - `begin` → `scope`

### jsonpiler 0.1

#### 0.1.11

- Added
  - Caching and reuse of string literals

- Changed
  - Generalized variable generation in assembly code
  - Names of internal structures

#### 0.1.10

- Fixed
  - Parser issues from the previous version

#### 0.1.9

- Added
  - Feature to dynamically adjust stack allocation

- Changed
  - Error message format

- Fixed
  - ~~Reduced parser overhead~~

#### 0.1.8

- Added
  - Function `global`

- Changed
  - Structure of the variable table
  - ~~`begin` introduces a scope~~

#### 0.1.7

- Added
  - Temporary register preservation with automatic save and restore

- Changed
  - ~~The main function returns `ExitCode`~~
  - Internal functions are included in the binary only when needed

#### 0.1.6

- Changed
  - Function `=` returns `null`
  - `$` supports more types

- Fixed
  - Documentation build error on docs.rs

#### 0.1.5

- Added
  - `CHANGELOG.md`

#### 0.1.4

- Fixed
  - Typo in the previous crate

#### 0.1.3

- Changed
  - Object entries preserve insertion order
  - Object value evaluation order follows insertion order

#### 0.1.2

- Added
  - Mermaid diagram in `README.md`

- Fixed
  - Bug affecting expression evaluation order

#### 0.1.0 ~ 0.1.1

- Migrated from the previous crate.
