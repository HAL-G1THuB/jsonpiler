# Specification

Specification of the Jsonpiler language.

## Syntax

The syntax is based on the JSON specification.  
See: [JSON specification](https://www.rfc-editor.org/info/rfc8259)

## Types

### Primitive Types

- **Null**: Represents the absence of a value.
- **Bool**: Represents a 8-bit boolean value, either `true` or `false`.
- **Int**: Represents a 64-bit integer number.
- **Float**: Represents a 64-bit floating-point number.
- **String**: Represents a sequence of characters enclosed in double quotes.

### Composite Types

- **Array**: Represents an ordered collection of values.

#### Subtypes of Object

- **HashMap**: Represents a collection of key-value pairs.

```json
{ "key": "value" }
```

- **Sequence**: Represents an ordered sequence of instructions.

```json
{
  "message": ["123", "456"],
  "+": [1, 2]
}
```

- **TypeAnnotations**: Represents a type annotation for a variable or function.

```json
{
  "i": "Int",
  "x": "String" ,
  "return": "String"
}
```

## Evaluation

A Jsonpiler program is represented as a single JSON value.

Each JSON value is evaluated independently, except in the case of arrays (`[]`) and objects (`{}`).

### Arrays

- Each element in the array is evaluated sequentially.
- The result is a new array containing all evaluated elements, preserving their order.

```json
[1, {"+": [2, 3]}, "text"]
// => [1, 5, "text"]
```

### Objects

- Each key in the object is interpreted as a function name.
- Each value is evaluated and passed as the argument to the function.
- If the key matches a registered built-in function, that is used; otherwise, a user-defined function is looked up.
- Multiple key-function entries are supported and evaluated in insertion order.
- The result of the last function call is returned as the final value.

```json
{
  "message": ["123", "456"],
  "+": [1, 2]
}
// => evaluates "message" then "+", returns result of "+"
```

## Exits

If the entire program evaluates to Int, it returns it as an exit code.
The exit code returned by jsonpiler is a 32-bit signed integer that is the exit code of the generated .exe file.
If a compilation error occurs or the executable file is not generated, the exit code is 1.

## Encoding

This program supports UTF-8.

## JSPL Language Specification

### Overview

This language builds upon JSON structure with lightweight syntactic sugar and extensions such as function calls and identifiers. It also allows top-level block syntax (`JSPL`) without requiring surrounding `{}` braces.

### Top-Level Syntax

- In addition to standard JSON (`parse_json`), a more flexible syntax (`parse_block`) is supported.
- In JSPL syntax, top-level `{}` can be omitted.
- Function-style calls using identifiers with arguments (e.g., `ident(arg1, arg2)`) are allowed.

---

### Syntax Details

#### Value

A value can be one of the following:

- String: `"abc"`
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

### Number

- Numbers can be integers or floating-point values.
- A leading `-` is allowed to indicate a negative number.
- `+` is **not allowed**.
- Leading `0` in integers is **prohibited** (e.g., `0123` is an error).
- Floating-point (`123.45`) and exponential notation (`1.23e+10`) are supported.

  - Exponents require `e` or `E` followed by optional `+` or `-` and digits.

#### BNF Syntax for Numbers

```bnf
number ::= '-'? int ('.' [0-9]+)? ([eE] [+-]? [0-9]+)?
int    ::= '0' | [1-9][0-9]*
```

---

### Identifier

- Identifiers can include any printable ASCII character in the range `!` to `~` (0x21–0x7E), **excluding** the following symbols:

  - `(`, `)`, `[`, `]`, `{`, `}`, `,`, `"`
- The following words are **reserved** and cannot be used as identifiers:

  - `true`, `false`, `null`
  - Names starting with `$` (e.g., `$name`) have reserved meaning

Internally, it is treated as an abbreviation of String.

---

### String

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

### Array

```json
[ "item1", 42, { "key": "value" } ]
```

- A list of values enclosed in square brackets `[]`.
- Values are separated by commas.
- Empty arrays like `[]` are allowed.

---

### Object

```text
key1(value1)
key2(42)
```

```text
{ key1: value1, key2: 42 }
```

- Keys must be strings or identifiers.
- An empty object `{}` is generally not acceptable when evaluated as a function sequence.

---

### JSPL Extended Syntax

#### Omitted `{}` for Top-Level Blocks

```js
key: "value"
list: [1, 2, 3]
```

→ Equivalent JSON:

```json
{
  "key": "value",
  "list": [1, 2, 3]
}
```

### Triple Syntax: Value + Identifier + Value

```js
1 + 10
```

→

```json
{ "+": [1, 10] }
```

### Identifier Function Syntax

```js
sum(1, 2, 3)
```

→

```json
{ "sum": [1, 2, 3] }
```

```js
abs: -1
```

```json
{ "abs": -1 }
```

### `$name` Notation

```js
$name
```

→

```json
{ "$": "name" }
```

---

#### Comment

```text
# comment
```

## Parser Error Rules

- Invalid string escape → Error
- Strings containing control characters → Error
- Integers with multiple digits starting with `0` → Error
- Floating-point number with no digits after `.` → Error
- Exponent with no digits after `e`/`E` → Error
- Non-string object keys → Error
- Unexpected trailing characters at top level → `Unexpected trailing characters`

---
