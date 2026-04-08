# 型

## プリミティブ型

- **Null**：値の欠如。概念としてはUnit値に近い。
- **Bool**：8 ビットのブール値。`true` または `false`。
- **Int**：64 ビットの整数値。
- **Float**：64 ビットの浮動小数点数。
- **Str**：ダブルクオートで囲まれたUTF-8文字列。

## 複合型

- **Array**：順序付きの値の集合。

### Object のサブタイプ

- **HashMap**：キーと値のペアの集合。

```json
{ "key": "value" }
```

- **Block**：命令の順序付き列。

```json
{
  "message": ["123", "456"],
  "+": [1, 2]
}
```

- **TypeAnnotations**：変数や関数に対する型注釈。

```json
{
  "i": "Int",
  "x": "Str",
  "return": "Str"
}
```
