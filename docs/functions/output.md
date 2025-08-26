# Output

## message

```json
{"message": [{"title": "String"}, {"text": "String"}]} -> "Null"
```

```text
message("title", "text")
```

Displays a message box.  
The first argument is the title; the second is the body text.  

## print

```json
{"print": ["String", "..."]} -> "Null"
```

```text
print(string, ...)
```

Prints the given string to the console.
Currently redirects and pipes are not supported.

```json
{"print": ["Hello, World!"]} => null
```
