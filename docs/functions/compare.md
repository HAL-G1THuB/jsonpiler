# Comparison

## !=

```jspl
Int != Int... -> Bool
Float != Float... -> Bool
Str != Str -> bool
```

Returns `true` if all arguments are not equal, `false` otherwise.

```jspl
1 != 1 => false
```

## ==

```jspl
Int == Int... -> Bool
Float == Float... -> Bool
Str == Str -> bool
```

Returns `true` if all arguments are equal, `false` otherwise.

```jspl
1 == 1 => true

"123" == "123" => true
"123" == "321" => false
```

## <

```jspl
Int < Int... -> Bool
Float < Float... -> Bool
```

Returns `true` if the arguments are in strictly increasing order, `false` otherwise.

```jspl
2 < 1 => false
```

## <=

```jspl
Int <= Int... -> Bool
Float <= Float... -> Bool
```

Returns `true` if the arguments are in increasing order, `false` otherwise.

```jspl
1 <= 2 => true
```

## >=

```jspl
Int >= Int... -> Bool
Float >= Float... -> Bool
```

Returns `true` if the arguments are in decreasing order, `false` otherwise.

```jspl
2 >= 1 => true
```

## >

```jspl
Int > Int... -> Bool
Float > Float... -> Bool
```

Returns `true` if the arguments are in strictly decreasing order, `false` otherwise.

```jspl
2 > 1 => true
```
