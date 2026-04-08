# Output

## message

```jspl
message(title: Str, text: Str) -> Null
```

Displays a message box.  
The first argument is the title; the second is the body text.  

## confirm

```jspl
confirm(title: Str, text: Str) -> Bool
```

Displays a confirmation dialog box with "Yes" and "No" buttons.
Returns `true` if the user clicks "Yes", and `false` if the user clicks "No".

```jspl
if(confirm("Confirmation", "Do you want to continue?"), print("Running..."))
```

## print

```jspl
print(Str, ...) -> Null
```

Prints the specified string to the console immediately when the arguments are evaluated.
Unlike normal functions, print outputs values as soon as they are ready,
rather than after the entire expression is evaluated.
Redirects and pipes are supported.

```jspl
print("Hello, ", "World!", "\n") => null
```

## input

```jspl
input() -> Str
```

Redirection and pipes are supported.

When the input is redirected or piped, data is read in 4 KB chunks by default,
and any remaining data is retained internally for subsequent reads.

```jspl
name = input()
print("Hello, ", name, "\n")
```
