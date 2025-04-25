# CHANGELOG

## jsonpiler 0.1

## 0.1.11

- Added functionality to cache and reuse string literals.
- Generalized variable generation in assembly.
- Significantly renamed structures for clarity.

### 0.1.10

- Significantly improved parser slowness caused by changes in the previous version

### 0.1.9

- Added the ability to dynamically change stack allocations (currently unused)
- The format of the error statement was fixed.
- Improved parser overhead.

### 0.1.8

- Created a new function `global`.
- Changed the structure of the variable table.
- `begin` now introduces a new scope.

### 0.1.7

- main function now returns `ExitCode` instead of `!`
- Added functionality to store temporary values in registers,
 saving and restoring previous values as needed.
- Added functionality to include internal functions in the binary only when needed.

### 0.1.6

- "=" function now returns null.
- Fixed problem with docs.rs documents failing to build.
- "$" for more assignable values.

### 0.1.5

- Added `CHANGELOG.md` to track updates.

### 0.1.4

- Fixed a typo in the previous crate.

### 0.1.3

- Object entries now preserve their insertion order.
- Evaluation of object values now follows insertion order as well.

### 0.1.2

- Added a Mermaid diagram to `README.md`.
- Fixed a bug affecting the evaluation order of expressions.

### 0.1.0 ~ 0.1.1

- Transition from previous crate.
