# JSPL Language Specification

## Overview

This language builds upon JSON structure with syntactic sugar and extensions such as function calls and identifiers. It also allows top-level block syntax (`JSPL`) without requiring surrounding `{}` braces.

## Top-Level Syntax

- In addition to standard JSON (`parse_json`), a more flexible syntax (`parse_block`) is supported.
- In JSPL syntax, top-level `{}` can be omitted.
- Function-style calls using identifiers with arguments (e.g., `ident(arg1, arg2)`) are allowed.

---

## Syntax Details

### Value

A value can be one of the following:

- Str: `"abc"`
- Number: `123`, `-10`, `1.23`, `2e5`
- Boolean: `true`, `false`
- Null: `null`
- Array: `[ val1, val2, ... ]`
- Object: `{ "key": value, ... }`
- Special syntaxes:

  - Identifier: `someIdent` → `"someIdent"`
  - Identifier function: `someFunc(arg1, arg2)` → `{ "someFunc": [ arg1, arg2 ] }`
  - Plain identifier: `$name` → `{ "$": "name" }`
  - Triple syntax: `val1 ident val2` → `{ ident: [ val1, val2 ] }`

---

## Number

- Numbers can be integers or floating-point values.
- A leading `-` is allowed to indicate a negative number.
- `+` is **not allowed**.
- Leading `0` in integers is **prohibited** (e.g., `0123` is an error).
- Floating-point (`123.45`) and exponential notation (`1.23e+10`) are supported.

  - Exponents require `e` or `E` followed by optional `+` or `-` and digits.

### BNF Syntax for Numbers

```text
number ::= '-'? int ('.' [0-9]+)? ([eE] [+-]? [0-9]+)?
int    ::= '0' | [1-9][0-9]*
```

---

## Identifier

- Identifiers can include any printable ASCII character in the range `!` to `~` (0x21–0x7E), **excluding** the following symbols:

  - `(`, `)`, `[`, `]`, `{`, `}`, `,`, `"`
- The following words are **reserved** and cannot be used as identifiers:

  - `true`, `false`, `null`
  - Names starting with `$` (e.g., `$name`) have reserved meaning

Internally, it is treated as an abbreviation of Str.

---

## Str

- UTF-8 strings enclosed in double quotes `"..."`.
- Supports escape sequences using backslashes:

| Sequence | Meaning                                        |
| -------- | ---------------------------------------------- |
| `\"`     | Double quote                                   |
| `\\`     | Backslash                                      |
| `\/`     | Forward slash                                  |
| `\b`     | Backspace                                      |
| `\f`     | Form feed                                      |
| `\n`     | Newline                                        |
| `\r`     | Carriage return                                |
| `\t`     | Tab                                            |
| `\uXXXX` | Unicode code point in hex (converted to UTF-8) |

- Unescaped control characters or newlines are **not allowed**.

---

## Array

```json
[ "item1", 42, { "key": "value" } ]
```

- A list of values enclosed in square brackets `[]`.
- Values are separated by commas.
- Empty arrays like `[]` are allowed.

---

## Object

```jspl
key1(value1)
key2(42)
```

```jspl
{ key1: value1, key2: 42 }
```

- Keys must be strings or identifiers.
- An empty object `{}` is generally not acceptable when evaluated as a function sequence.

---

## JSPL Extended Syntax

### Omitted `{}` for Top-Level Blocks

```jspl
key: "value"
list: [1, 2, 3]
```

→

```json
{
  "key": "value",
  "list": [1, 2, 3]
}
```

### Multiple elements on a single line in an object

```jspl
key1: "value"; key2: 42
```

→

```json
{ "key1": "value", "key2": 42 }
```

### Infix notation (Applicable to any function)

```jspl
1 + 20 + 300
```

→

```json
{ "+": [1, 20, 300] }
```

### Identifier Function Syntax

```jspl
sum(1, 2, 3)
```

→

```json
{ "sum": [1, 2, 3] }
```

```jspl
abs: -1
```

```json
{ "abs": -1 }
```

### `$name` Notation

```jspl
$name
```

→

```json
{ "$": "name" }
```

---

#### Comment

```jspl
# comment
```

## Parser Error Rules

- Invalid string escaping
- `Str` containing control characters
- Multi-digit `Int` starting with `0`
- `Float` without a digit after `.`
- `Float` without an exponent after `e`/`E`
- `Object` key not being `Str`
- Remaining invalid top-level input

---
