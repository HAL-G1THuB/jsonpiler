# CHANGELOG

## jsonpiler 0.1

### jsonpiler 0.1.7

- main function now returns `ExitCode` instead of `!`
- Added functionality to store temporary values in registers,
 saving and restoring previous values as needed.
- Added functionality to include internal functions in the binary only when needed.

### jsonpiler 0.1.6

- "=" function now returns null.
- Fixed problem with docs.rs documents failing to build.
- "$" for more assignable values.

### jsonpiler 0.1.5

- Added `CHANGELOG.md` to track updates.

### jsonpiler 0.1.4

- Fixed a typo in the previous crate.

### jsonpiler 0.1.3

- Object entries now preserve their insertion order.
- Evaluation of object values now follows insertion order as well.

### jsonpiler 0.1.2

- Added a Mermaid diagram to `README.md`.
- Fixed a bug affecting the evaluation order of expressions.
