# Output

## message

```json
{"message": [{"title": "String"}, {"text": "String"}]} -> "Null"
```

```jspl
message("title", "text")
```

Displays a message box.  
The first argument is the title; the second is the body text.  

## print

```json
{"print": ["String", "..."]} -> "Null"
```

```jspl
print(string, ...)
```

Prints the specified string to the console.
Redirects and pipes are supported.

```json
{"print": ["Hello, World!"]} => null
```
