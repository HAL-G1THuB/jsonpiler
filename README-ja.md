# Jsonpiler — JSON構文プログラミング言語

[英語(English)](https://github.com/HAL-G1THuB/jsonpiler/blob/main/README-ja.md)

**Jsonpiler** は **JSON** を文法として使うプログラミング言語のコンパイラ兼実行環境です。
JSONで書かれたプログラムを **x86\_64 Windows PE** 形式の機械語に変換し、リンクして実行します。
Jsonpiler は、その中間表現（IR）から Windows 用 PE を出力することに特化した **自前実装のアセンブラとリンカ** を内蔵しています。

- GitHub: [https://github.com/HAL-G1THuB/jsonpiler](https://github.com/HAL-G1THuB/jsonpiler)
- Crates.io: [https://crates.io/crates/jsonpiler](https://crates.io/crates/jsonpiler)
- AI生成ドキュメント: [![badge](https://deepwiki.com/badge.svg)](https://deepwiki.com/HAL-G1THuB/jsonpiler)

> 🚨 **Windowsのみ (x64)** — Jsonpiler は 64ビット Windows を対象に、ネイティブ PE 実行ファイルを生成します。

---

## 更新情報

### 0.5.4

- .gitattributes に `.bin binary` を追加した。
- 0.5.0の重大なバグを修正
- 32bit 以上の値を使った一部の `mov` 命令でアセンブラがエラーになる問題を修正。
- **内蔵アセンブラとリンカ**: GNU `as`/`ld` は不要になりました。
- 一時的な `.s` や `.obj` ファイルは生成されません。
- SEH サポートは **一時的に無効化** されています。

詳細は **[CHANGELOG](https://github.com/HAL-G1THuB/jsonpiler/blob/main/CHANGELOG-ja.md)** を参照してください。

---

## 必要条件

外部ツールやライブラリは不要です。

**以下のシステムDLLが `C:\Windows\System32\` に存在する必要があります:**

- `kernel32.dll`
- `user32.dll`

標準的な Windows 環境ではすでに存在します。

---

## インストールと実行

```bash
cargo install jsonpiler

# JSON プログラムをコンパイルして実行
jsonpiler <input.json> [生成exeへの引数]
```

- `<input.json>` は UTF-8 である必要があります。
- 追加の引数は生成された実行ファイルに渡されます。

---

## 言語仕様・関数リファレンス

- **言語仕様 (Markdown)**: [https://github.com/HAL-G1THuB/jsonpiler/blob/main/docs/specification.md](https://github.com/HAL-G1THuB/jsonpiler/blob/main/docs/specification.md)
- **関数リファレンス (Markdown)**: [https://github.com/HAL-G1THuB/jsonpiler/blob/main/docs/functions.md](https://github.com/HAL-G1THuB/jsonpiler/blob/main/docs/functions.md)

---

## 使用例

準備済みサンプルは **`examples/`** にあります:
[https://github.com/HAL-G1THuB/jsonpiler/blob/main/examples](https://github.com/HAL-G1THuB/jsonpiler/blob/main/examples)

最小例:

```json
{ "=": ["a", "title"], "message": [{"$": "a"}, "345"], "+": [1, 2, 3] }
```

### 実行順序

- Jsonpiler プログラムは単一の JSON オブジェクトで構成され、キーは **順次評価** されます。
- `"="` は文字列 `"title"` を変数 `a` に代入します。
- `"message"` は `a` の値に `"345"` を連結して表示します。
- `"+"` は `1 + 2 + 3` を計算し、結果は **6** です。

プログラムの **最終式の値** はプロセスの **終了コード** になります。Cargo で実行すると次のように表示される場合があります:

```text
error: process didn't exit successfully: `jsonpiler.exe test.json` (exit code: 6)
```

これは Jsonpiler のエラーではなく、想定された動作です。

---

## エラー・警告の例

**入力:**

```json
{ "message": ["title", { "$": "doesn't_exist" }] }
```

**出力:**

```text
Compilation error: Undefined variables: `doesn't_exist`
Error occurred on line: 1
Error position:
{ "message": ["title", { "$": "doesn't_exist" }] }
                              ^^^^^^^^^^^^^^^
```

---

## 処理フロー概要

```mermaid
graph TD
  subgraph 読み込み
    A["file.json\n{ &quot;+&quot;: [1, 2] }"] --> B{Jsonpiler}
  end
  subgraph 解析
    B --o C["AST\nJson::Object([
      Json::String(&quot;+&quot;),
      Json::Array([
        Json::Int(1),
        Json::Int(2)
      ])])"]
    B --x PError[[ParseError]]
  end
  subgraph コンパイル
    C --x CError[[CompileError]]
    C --o E["アセンブラ IR
    ...Inst::MovQQ(Rax, 1),
    Inst::MovQQ(Rcx, 2),
    Inst::AddRR(Rax, Rcx)..."]
  end
  subgraph 評価
    C --o D["Json::Int(一時値)"]
    C --x EError[[TypeError or ArityError]]
    D -->|終了コード検出| E
  end
  subgraph アセンブル
    E --o G["バイナリ機械語
    [...0x48, 0x89, 0o201...]"]
    E --x AError[[InternalError]]
  end
  subgraph リンク
    G --o F["PE形式 (Portable Executable)"]
    G --x LError[[InternalError]]
  end
  subgraph 書き込み
    F --> H[file.exe]
  end
  subgraph 実行
    H --> Exec[(Execute)]
  end
  subgraph DLL
    S[C:\\Windows\\System32\\]
    KERNEL32[kernel32.dll]
    USER32[user32.dll]
    S --> KERNEL32 --> F
    S --> USER32 --> F
  end
```

---

## 注意事項

- 出力は Windows x64 向けのネイティブ **PE 実行ファイル** です。
- SEH は現在無効化されており、将来のリリースで再度有効になる可能性があります。
- Cargo 実行時に 0 以外の終了コードが返る場合は、プログラムの最終値によるものです。

---

## ライセンス

ライセンスはリポジトリで確認してください。

---

## 貢献について

Issues や PR は歓迎します。バグを発見した場合は、以下の情報を含めてください。

> 🚨 Windows x64 であることを確認してください。

- JSON プログラム（可能であれば最小限の再現例）
- Jsonpiler のバージョン
