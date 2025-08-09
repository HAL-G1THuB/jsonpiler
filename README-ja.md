# Jsonpiler - JSON文法プログラミング言語

**Jsonpiler**(**ジェイソンパイラー**)は  
Jsonとほぼ同様の文法であるプログラミング言語と、  
それを実行可能な.exeファイルに変換するコンパイラです。

このプログラムは、JSONベースのプログラムをGNUアセンブリに変換し、
さらにアセンブリににGNU AS、GNU LDを使用して結果を実行します。

[英語(English)](https://github.com/HAL-G1THuB/jsonpiler/blob/main/README.md)

- [GitHubリポジトリ](https://github.com/HAL-G1THuB/jsonpiler)  
- [Crates.io](https://crates.io/crates/jsonpiler)  
- [AI生成ドキュメント![badge](https://deepwiki.com/badge.svg)](https://deepwiki.com/HAL-G1THuB/jsonpiler)  
🚨 **Windowsでのみ作動します (x64)!** 🚨

## 変更履歴

### 0.4.2

- 新しい関数`concat`が追加された。この関数は文字列リテラル同士をリテラルを保ちながら結合するために使用される。
- Objectを3つの亜種に分割。
- **HashMap**: キーと値のペアのコレクションを表す。
- **Sequence**: 命令の順序付けられたシーケンスを表す。
- **TypeAnnotations**: 変数または関数の型アノテーションを表す。
- 四則演算関数の引数を与えない場合、エラーが出るようになった。
- `lambda`の引数を実装。
- `lambda`の返り値にできる型が豊富になった。
- `+`, `/`, `*`, `or`, `and`, `xor`の最低限の引数の数が2になった。
- `message`の返り値が`Null`になった。

[プロジェクトの歴史と計画](https://github.com/HAL-G1THuB/jsonpiler/blob/main/CHANGELOG-ja.md)

## 前提条件

**以下のツールがインストールされ、PATH環境変数で利用可能であることを確認してください:**

- `ld` (from MinGW-w64)  
- `as` (from MinGW-w64)  

**このプログラムを正常に動作させるためには、以下のDLLがC:Windows/System32/に存在する必要があります。**

- `kernel32.dll`  
- `user32.dll`  

## インストールと使用方法

```bash
cargo install jsonpiler
jsonpiler (input_json_file (UTF-8)) [arguments of .exe ...]
```

(input_json_file)`をコンパイルしたい実際のJSONファイルに置き換えてください。

## 関数一覧

[関数一覧 (マークダウン)](https://github.com/HAL-G1THuB/jsonpiler/blob/main/docs/functions.md)

## 言語仕様

[言語仕様 (マークダウン)](https://github.com/HAL-G1THuB/jsonpiler/blob/main/docs/specification.md)

## 例

[例](https://github.com/HAL-G1THuB/jsonpiler/blob/main/examples)

```json
{ "=": ["a", "title"], "message": [{"$": "a"}, "345"] }
```

**実行順序:**

jsonpilerのコードは、1つのJSONオブジェクトで構成される。

式は順番に評価される。

キー `"="` は文字列 `"title"` を変数 `a` に代入する。  

次に、`"message"` キーは `a` の値の後に文字列 `"345"` を続けたものを使ってメッセージを表示する。  

最後に、`"+"`キーは `1`、`2`、`3`の和を計算し、結果 `6` を生成する。

このプログラムは `{}` ブロックの最終値として `6` を返す。

このプログラムをcargoで動作するjsonpilerに読み込ませると、次のように表示される（上記のように6を返す）。

```plaintext
error: process didn't exit successfully： `jsonpiler.exe test.json` (exit code: 6)

```

これは予期せぬエラーではなく、正常な動作である。

## エラー、もしくは警告メッセージの形式

```json
{ "message": ["タイトル", { "$": "存在しない" }] }
```

```text
Compilation error: Undefined variables: `存在しない`
Error occurred on line: 1
Error position:
{ "message": ["title", { "$": "doesn't_exist" }] }
                              ^^^^^^^^^^^^^^^
```

## 実行のイメージ図

```mermaid
graph TD
  A[file.json] --> B{Jsonpiler}
  B -->|Parse| C([AST])
  C -->|本プログラムでコンパイル| D[file.s]
  D --> |GNU ASでアセンブル| E[file.obj]
  E --> |GNU LDでリンク| F[file.exe]
  S[C:\Windows\System32\] --> KERNEL32[kernel32.dll] --> F
  S --> USER32[user32.dll] --> F
  F --> Exec[(実行)]
```
