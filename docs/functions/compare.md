# Comparison

## !=

```jspl
Int != Int... -> Bool
Float != Float... -> Bool
Str != Str -> bool
```

Returns `true` if no adjacent arguments are equal.

```jspl
1 != 1 => false
```

## ==

```jspl
Int == Int... -> Bool
Float == Float... -> Bool
Str == Str -> bool
```

Returns `true` if all arguments are equal.

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

Returns `true` if arguments are in strictly increasing order.

```jspl
2 < 1 => false
```

## <=

```jspl
Int <= Int... -> Bool
Float <= Float... -> Bool
```

Returns `true` if arguments are in increasing order.

```jspl
1 <= 2 => true
```

## >=

```jspl
Int >= Int... -> Bool
Float >= Float... -> Bool
```

Returns `true` if arguments are in decreasing order.

```jspl
2 >= 1 => true
```

## >

```jspl
Int > Int... -> Bool
Float > Float... -> Bool
```

Returns `true` if arguments are in strictly decreasing order.

```jspl
2 > 1 => true
```
