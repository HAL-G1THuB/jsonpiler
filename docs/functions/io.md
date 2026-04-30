# Output

## message

```jspl
message(title: Str, text: Str) -> Null
```

Displays a message box.
The first argument is the title; the second is the message.

## confirm

```jspl
confirm(title: Str, text: Str) -> Bool
```

Displays a confirmation dialog with `"Yes"` and `"No"` buttons.
Returns `true` if `"Yes"` is selected, `false` if `"No"` is selected.

```jspl
if(confirm("Confirmation", "Do you want to continue?"), print("Running..."))
```

## print

```jspl
print(Str...) -> Null
```

Prints the specified string to the console as soon as arguments are evaluated.
Unlike normal functions,
outputs values as they become available
rather than after the entire expression is evaluated.
Supports redirection and piping.

```jspl
print("Hello, ", "World!", "\n") => null
```

## input

```jspl
input() -> Str
```

Reads a line from the console and returns it as a string.
Returns input when Enter is pressed.
When input is redirected or piped,
reads data in 4 KB chunks by default and retains remaining data for subsequent reads.

```jspl
name = input()
print("Hello, ", name, "\n")
```
