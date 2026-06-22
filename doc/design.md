## 方針

要件を次の順に分解して設計する。

1. 数式の意味論と入力文法を固定する。
2. 厳密値と保証付き近似値を独立した情報として扱う。
3. 有理数、代数的数、(\pi) の有理数倍、一般の形式式を、それぞれ無理なく扱える内部表現を定義する。
4. Rust core、Wasmアダプター、npm公開API、サンプルUIの依存方向を固定する。
5. 計算量制限、型付きエラー、決定性を公開APIの一部として定義する。
6. 最終設計を崩さず、実装難度の低い領域から段階的に完成させる。

主要な用語は以下の意味で用いる。

* **Exact Arithmetic** (`/ɪɡˈzækt əˈrɪθmətɪk/`, 厳密演算)
* **Arbitrary-Precision Arithmetic** (`/ˈɑːrbɪtreri prɪˈsɪʒən əˈrɪθmətɪk/`, 任意精度演算)
* **Algebraic Number** (`/ˌældʒəˈbreɪɪk ˈnʌmbər/`, 代数的数)
* **Interval Arithmetic** (`/ˈɪntərvəl əˈrɪθmətɪk/`, 区間演算)
* **Ball Arithmetic** (`/bɔːl əˈrɪθmətɪk/`, ボール演算)
* **Directed Rounding** (`/dəˈrektɪd ˈraʊndɪŋ/`, 方向付き丸め)
* **Abstract Syntax Tree** (`/ˈæbstrækt ˈsɪntæks triː/`, 抽象構文木 [AST])
* **Directed Acyclic Graph** (`/dəˈrektɪd eɪˈsaɪklɪk ɡræf/`, 有向非巡回グラフ [DAG])
* **WebAssembly** (`/ˌweb əˈsembli/`, Web向けバイナリ命令形式 [ウェブアセンブリ、Wasm]; 英語: web + assembly)
* **Data Transfer Object** (`/ˈdeɪtə ˈtrænsfɜːr ˈɒbdʒekt/`, データ転送オブジェクト [DTO])

この文書では、次の二種類を明示的に分ける。

* **公開契約**: 入力文法、意味論、公開API型、DTO、エラー分類、計算量制限、保証の意味。これらは現行実装が利用者に約束する契約となる。
* **実装詳細**: 現時点のcrate候補、内部アルゴリズム、キャッシュ戦略、内部データ構造の最適化。これらは公開契約を壊さない範囲で変更できる。

現行実装については、公開契約の正本を [`public-contract.md`](public-contract.md) に、実装状況と未完了領域を [`implementation-status.md`](implementation-status.md) に分離して記録する。この `design.md` は最終設計の目標を記述し、現行実装がすべての章を満たしていることを意味しない。

特に、公開API、データモデル、エラー型、モジュール境界は試作段階でも暫定設計にしない。設計ミスが分かった場合は、場当たり的な互換層を追加せず、公開契約そのものを再確認して修正する。

---

## 1. 最重要の設計判断

### 1.1 数値型を単純な「昇格階層」にしない

次のような一方向の昇格は採用しない。

```text
Integer
  → Rational
  → Algebraic
  → BigFloat
```

この構造では、次の値を正しく表現できない。

* (\pi/6) は代数的数ではないが厳密値である。
* (\sin(\pi/6)) は有理数 (1/2) へ戻る。
* (\sqrt{2}+\pi) は代数的数でも (\pi) の有理数倍でもない。
* (\sin(1)) は通常の閉形式へ変換できなくても、形式式として厳密に保持できる。

したがって、内部値は次の二軸で管理する。

```text
厳密な意味を持つ式
    +
その式について認識できた特殊な厳密表現
    +
独立して計算された保証区間
```

概念的には次の構造となる。

```rust
pub struct EvaluatedValue {
    exact_expression: ExactExpression,
    recognized_exact: RecognizedExact,
    certified_enclosure: CertifiedEnclosureState,
}

pub struct ExactExpression {
    _private: (),
}

pub enum CertifiedEnclosureState {
    NotRequested,
    Available(CertifiedInterval),
    Unavailable,
}

pub enum RecognizedExact {
    Rational(Rational),
    RealAlgebraic(RealAlgebraic),
    RationalPiMultiple(Rational),
    GeneralSymbolic,
}
```

`recognized_exact` は値の本体ではなく、厳密式に対して得られた追加情報である。計算量制限によって代数的数への変換を断念しても、`exact_expression` は失われない。

これにより、要件中の「厳密式を保持したまま証明付き近似へ切り替える」を自然に実装できる。

---

### 1.2 近似値を厳密値の代替にしない

内部では次の二つを完全に分離する。

```text
ExactExpression
CertifiedEnclosure
```

例えば `sin(1)` の結果は次のようになる。

```text
exact_expression:
    sin(1)

recognized_exact:
    GeneralSymbolic

certified_enclosure:
    [0.84147098480789650665..., 0.84147098480789650666...]
```

科学表記は保証区間から生成する。表示用に丸めた小数を、その後の計算へ再入力してはならない。

`Ans`、メモリー、履歴から再利用する値も、表示文字列ではなく厳密式を保持する。

---

### 1.3 実数モードを最初の意味論とする

初期版では計算領域を明示的に実数とする。

```rust
pub enum EvaluationDomain {
    Real,
}
```

複素数は将来、次のように別モードとして追加する。

```rust
pub enum EvaluationDomain {
    Real,
    Complex,
}
```

実数モードでは、例えば以下を型付き定義域エラーとする。

* `sqrt(-1)`
* `ln(0)`
* `asin(2)`
* `1 / 0`
* `0 ^ 0`
* `0 ^ -1`

`NaN` や無限大を通常の数値結果として外部へ漏らさない。

---

## 2. リポジトリ構成

過剰にcrateを分割せず、プラットフォーム境界だけを明確に分ける。

```text
workspace/
├── Cargo.toml
├── crates/
│   ├── calculator-core/
│   │   └── src/
│   │       ├── syntax/
│   │       ├── expression/
│   │       ├── number/
│   │       ├── simplify/
│   │       ├── evaluate/
│   │       ├── format/
│   │       ├── session/
│   │       ├── error.rs
│   │       ├── limits.rs
│   │       └── lib.rs
│   ├── calculator-wasm/
│   │   └── src/
│   │       ├── dto.rs
│   │       ├── convert.rs
│   │       └── lib.rs
│   ├── calculator-cli/
│   │   └── src/main.rs
│   ├── calculator-wasi/
│   │   └── src/main.rs
│   └── xtask/
│       └── src/main.rs
├── packages/
│   └── calculator/
│       ├── src/
│       │   ├── index.ts
│       │   ├── direct.ts
│       │   ├── worker.ts
│       │   ├── presentation.ts
│       │   └── generated/
│       └── package.json
└── examples/
    ├── vanilla-web/
    └── react/
```

依存関係は次のDAGに固定する。

```text
sample UI
    ↓
npm TypeScript facade
    ↓
calculator-wasm
    ↓
calculator-core

calculator-cli
    ↓
calculator-core

calculator-wasi
    ↓
calculator-core
```

`calculator-core` からWasm、JavaScript、DOM、CLI、標準入出力を参照しない。

これは、coreを `#![no_std]` とし、プラットフォーム依存処理を外層へ分け、エラーと表示を分離するという既存の開発方針に一致する。([Zenn][1])

公開物のmetadataは次で統一する。

| 項目 | 値 |
| ---- | -- |
| author | `bem130` |
| license | `MIT` |
| license file | repository rootの `LICENSE` |

Cargo workspace内の公開crate、npm package、生成物に含めるpackage metadataは、この表と矛盾してはならない。npmの `license` fieldは `MIT`、Cargo manifestの `license` fieldも `MIT` とし、license本文はrootの `LICENSE` を正本とする。

初期ターゲットは次とする。

| target | 所有crate/package | 境界で扱うもの |
| ------ | ---------------- | ------------- |
| native CLI | `calculator-cli` | `std`、コマンドライン引数、標準入出力、終了コード |
| WASI CLI | `calculator-wasi` | `wasm32-wasip2` の標準入出力、終了コード |
| Web library | `calculator-wasm` + `packages/calculator` | `wasm-bindgen`、JavaScript object、worker |
| sample UI | `examples/*` | DOM、ARIA、keyboard、clipboard |

`calculator-core` の関数は、同じ引数と同じ `EvaluationContext` の観測不能な内部状態に対して同じ結果を返す。環境変数、時刻、乱数、ロケール、CPU機能、cacheのwarm/cold状態によって、値・エラー種別・計算量制限判定が変化してはならない。

### `calculator-core` のcrate属性

```rust
#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;
```

ここでいう「純粋なRust」は、少なくとも次を契約とする。

* production用の推移的依存関係にC/C++などのFFIを含まない。
* core自身は `std`、OS、DOM、JavaScript APIに依存しない。
* core自身では `unsafe` を禁止する。
* 時刻、乱数、環境変数、ロケールを計算結果に使用しない。
* `f32`、`f64` を数値意味論へ使用しない。

`calculator-core` の公開型で `String`、`Vec`、`Box` を使う場合、それらは `alloc::{string::String, vec::Vec, boxed::Box}` を意味する。`std` featureを追加する場合でも、`std::error::Error` 実装、追加の変換実装、開発用diagnosticに限定し、計算意味論や公開DTOの形を変えてはならない。

CIでは `calculator-core` について少なくとも次を検査する。

```text
cargo check -p calculator-core --no-default-features
cargo test -p calculator-core --no-default-features
```

---

## 3. core内部の処理パイプライン

```text
UTF-8入力
    ↓
字句解析
    ↓
Source AST
    ↓
意味論的lowering
    ↓
Exact Expression DAG
    ↓
厳密簡約
    ↓
厳密表現の認識
    ↓
保証区間の評価
    ↓
科学表記の確定
    ↓
Presentation Tree
```

### 3.1 Source ASTとExact Expression DAGを分ける

Source ASTは、利用者が入力した構造とエラー位置を保持する。

```text
入力:
    -2^2
```

```text
Source AST:
    Negate(
        Power(2, 2)
    )
```

Exact Expression DAGは、意味論上等しい部分式を共有し、正規化された演算構造を保持する。

```rust
pub struct ExprId(u32);
pub struct ExprListId(u32);
pub struct RationalId(u32);

pub enum ExpressionNode {
    Rational(RationalId),
    Constant(Constant),
    Add(ExprListId),
    Multiply(ExprListId),
    Divide {
        numerator: ExprId,
        denominator: ExprId,
    },
    Power {
        base: ExprId,
        exponent: ExprId,
    },
    Function {
        function: Function,
        argument: ExprId,
    },
}
```

```rust
pub enum Constant {
    Pi,
    Euler,
}

pub enum Function {
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Sqrt,
    Exp,
    Log,
}

pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Power,
}
```

Source spanをDAGノードへ直接埋め込まない。同じ厳密部分式が複数の入力位置から共有される可能性があるため、spanは別の対応表で管理する。

---

### 3.2 入力文法で固定すべき仕様

| 項目        | 採用する意味               |
| --------- | -------------------- |
| `^`       | 右結合                  |
| 単項マイナス    | 累乗より優先度を低くする         |
| `-2^2`    | `-(2^2) = -4`        |
| `2^-3`    | `2^(-3) = 1/8`       |
| 小数点       | coreでは `.` のみ        |
| `1.25`    | 最初から `5/4`           |
| `1.2e-3`  | 最初から `3/2500`        |
| `pi`, `π` | 同じ厳密定数               |
| 逆三角関数     | `asin`、`acos`、`atan` |
| `sin^-1`  | 逆関数としては受理しない         |
| `%`       | 後置演算子として `x / 100`    |
| 角度単位      | requestの意味論設定として明示   |
| 暗黙乗算      | `2π`、`2(3+4)` を許可する |
| `eval`    | 一切使用しない              |

電卓特有の `100 + 10% = 110` のような文脈依存パーセントは、式評価器へ混ぜない。後述するsession層の `PercentPolicy` で処理する。

上の表だけでは公開構文契約として不足するため、初期版の文法は次で固定する。

```text
Expression        = Sum
Sum               = Product (("+" | "-") Product)*
Product           = Prefix (("*" | "/" | ImplicitMultiply) Prefix)*
Prefix            = ("+" | "-") Prefix | Percent
Percent           = Power ("%")*
Power             = Primary ("^" Prefix)?
Primary           = Integer
                  | Decimal
                  | Constant
                  | FunctionCall
                  | "(" Expression ")"
FunctionCall      = Identifier "(" Expression ("," Expression)? ")"
ImplicitMultiply  = gap between a left primary-like expression
                    and a right primary-like expression
```

この文法により、次の解析を公開契約とする。

| 入力        | 意味                         |
| --------- | -------------------------- |
| `2^3^2`   | `2^(3^2)`                  |
| `-2^2`    | `-(2^2)`                   |
| `2^-3`    | `2^(-3)`                   |
| `2/3π`    | `(2 / 3) × π`              |
| `2(3+4)`  | `2 × (3 + 4)`              |
| `(1+2)(3+4)` | `(1 + 2) × (3 + 4)`     |
| `50%`     | `50 / 100`                 |
| `(2^3)%`  | `(2^3) / 100`              |
| `sin 30`  | parse error                |
| `sin(30)` | function call              |
| `log(8,2)` | base-2 logarithm          |
| `ln(e)`   | natural logarithm          |
| `exp(3,2)` | base-2 exponential, `2^3` |

暗黙乗算は、明示的な `*` および `/` と同じ優先順位で左結合とする。関数呼び出しは必ず括弧を要求する。これにより、UIのボタン入力は `sin(` のような明示的な構造を生成し、文字列APIと同じparserを通せる。

lexerは次の契約に固定する。

| 分類 | 仕様 |
| ---- | ---- |
| 整数 | ASCII digit列。先頭 `+` / `-` はliteralではなく単項演算子 |
| 小数 | `digits "." digits`。`.5` と `1.` はparse error |
| 指数表記 | `integer_or_decimal ("e" | "E") ("+" | "-")? digits` |
| 空白 | token間のASCII whitespaceと一般的なUnicode whitespaceを無視する。ただし数値literal内部の空白は不可 |
| 定数 | `pi` と `π` は `Constant::Pi`、`e` は `Constant::Euler` |
| 関数名 | ASCII alphabetic identifierだけを関数名として受理し、未知名はparse error |
| 予約語 | `nan`、`inf`、`infinity`、`undefined`、`null` は数値として受理しない |

関数は原則として単項関数である。ただし `log(argument, base)` と `exp(exponent, base)` は2引数形式を受ける。`ln(argument)` は底 `e` の自然対数である。底を省略した `log(argument)` は受理しない。`max` / `min`、ユーザー定義関数は受理しない。

source位置はUTF-8 byte offsetを正本とし、parse errorのspanは「そのerrorを確定できた最小のtokenまたはtoken間位置」を指す。Wasm DTOでは同じspanに対応するUTF-16 code unit offsetを追加するが、core内部の正本は常にUTF-8である。

---

### 3.3 角度単位

三角関数カーネルの内部意味論はラジアンへ統一する。ただし、公開式言語の `AngleUnit` は、三角関数の入力角と逆三角関数の出力角の両方へ適用する。

```rust
pub enum AngleUnit {
    Radian,
    Degree,
    Gradian,
}
```

意味論的loweringでは、source-levelの関数を次のように内部ラジアン関数へ変換する。

```text
sin_u(x)  = sin_rad(to_radians(x, u))
cos_u(x)  = cos_rad(to_radians(x, u))
tan_u(x)  = tan_rad(to_radians(x, u))

asin_u(x) = from_radians(asin_rad(x), u)
acos_u(x) = from_radians(acos_rad(x), u)
atan_u(x) = from_radians(atan_rad(x), u)
```

例えばdegreeモードの `sin(30)` は、意味論的loweringで次へ変換する。

```text
sin_rad(30 × π / 180)
```

同じdegreeモードの `asin(1/2)` は次へ変換する。

```text
asin_rad(1/2) × 180 / π
= 30
```

この変換でも `π` を浮動小数点へ変換しない。`AngleUnit` は単なる表示設定ではなく式の意味論設定であるため、同じ入力文字列でも角度単位が異なれば厳密式が異なり得る。

評価結果のmetadataには、使用した角度単位を含める。履歴や `Ans` へ保存する値は、lowering後の厳密式と、入力時の `SemanticSettings` を一緒に保持する。

---

## 4. 厳密数値表現

### 4.1 任意精度整数と有理数

```rust
pub struct Integer {
    inner: BigIntBackend,
}

pub struct BigIntBackend {
    _private: (),
}

pub struct PositiveInteger {
    inner: Integer,
}

pub struct Rational {
    numerator: Integer,
    denominator: PositiveInteger,
}
```

`Rational` は生成時に必ず次の不変条件を満たす。

```text
denominator > 0
gcd(|numerator|, denominator) = 1
0 の分母は常に 1
```

第三者crateの `BigInt` 型を公開APIへ直接露出させない。将来バックエンドを変更できるよう、privateなwrapperを通す。

`num-bigint` は `default-features = false` で `std` を無効化でき、`alloc` を用いた任意精度整数として利用できる。第一候補として妥当である。([Docs.rs][2])

```toml
[dependencies]
num-bigint = { version = "0.4", default-features = false }
num-integer = { version = "0.1", default-features = false }
num-traits = { version = "0.2", default-features = false }
```

### 小数リテラルの変換

例えば入力が次の場合を考える。

```text
12.3400e-3
```

文字列から直接、

```text
123400 × 10^(-4) × 10^(-3)
```

を構築し、

```text
617 / 50000
```

へ約分する。

途中で `f64` を経由させない。

有限小数として厳密表示できる条件は、既約分数の分母の素因数が (2) と (5) だけであることである。

---

### 4.2 実代数的数

実代数的数は次の組で表現する。

```rust
pub struct RealAlgebraic {
    minimal_polynomial: PrimitivePolynomial,
    real_root_index: u32,
    isolating_interval: RationalInterval,
}

pub struct PrimitivePolynomial {
    coefficients_low_to_high: Vec<Integer>,
}
```

不変条件は以下とする。

1. 多項式係数は整数である。
2. 係数の最大公約数は1である。
3. 最高次係数は正である。
4. 多項式は (\mathbb{Q}) 上既約である。
5. `real_root_index` は実根を昇順に並べた位置である。
6. `isolating_interval` は対象の実根をただ一つだけ含む。
7. 区間端点自身は根でない。

`real_root_index` は0始まりとする。例えば最小の実根は `0`、二番目の実根は `1` である。このindexは内部正規化用であり、初期公開APIでは直接serializeしない。

`RationalInterval` は有理数の下端・上端を持つ閉区間として保持する。

```rust
pub struct RationalInterval {
    lower: Rational,
    upper: Rational,
}
```

ただし実代数的数の隔離区間として使う場合は、追加不変条件として `lower < upper` かつ両端点が根ではないことを要求する。端点が根ではないため、根の一意性判定において開区間・閉区間の差は生じない。

例えば (\sqrt{2}) は概念的に次で表す。

```text
minimal polynomial:
    x² - 2

root index:
    正の実根

isolating interval:
    [1, 2]
```

最小多項式と隔離区間による表現は、既存の厳密代数計算系でも採用されている。FLINTの `qqbar` も、既約最小多項式と根を一意に識別する隔離区間を用いる。ただし、次数 (m) と (n) の数同士の演算では次数 (mn) の消去多項式が現れ得るため、演算が高価になる。([Flint Library][3])

必要なアルゴリズムは次の通りである。

* 整数多項式の内容除去
* 多項式GCD
* square-free分解
* Sturm列による実根個数判定
* 実根隔離
* resultantによる加減乗除
* 多項式因数分解
* 数値区間と正確な符号判定による対象因子の選択
* 最小多項式への正規化

これは全体で最も難しい部分である。最初から無制限に実行せず、次の上限を適用する。

```text
max_algebraic_degree
max_polynomial_coefficient_bits
max_resultant_degree
max_factorization_work
max_root_isolation_steps
```

上限超過時はエラーにせず、可能な場合は次へフォールバックする。

```text
ExactExpression:
    元の厳密式を保持

RecognizedExact:
    GeneralSymbolic

CertifiedEnclosure:
    保証区間を計算
```

---

### 4.3 根号

`sqrt(8)` のような式は、一般代数的数へ直ちに変換する前に専用簡約を行う。

```text
sqrt(8)
= sqrt(4 × 2)
= 2sqrt(2)
```

ただし、巨大整数の完全因数分解を常に要求してはならない。

根号簡約は段階的に行う。

1. 完全平方判定
2. 小さい素因数の除去
3. 既知の平方因子の抽出
4. 計算量上限内なら完全なsquare-free分解
5. 上限超過なら未簡約根号を厳密に保持

「簡約できなかった」と「簡約不能であることを証明した」は区別する。

---

### 4.4 (\pi) の有理数倍

専用表現を用意する。

```rust
pub struct RationalPiMultiple {
    coefficient: Rational,
}
```

例えば次を構造的に認識する。

```text
π / 6
3π / 4
-11π / 7
```

数値近似が (\pi/6) に近いという理由では認識しない。

三角関数では、有理数係数を周期に従って厳密に剰余化する。

```text
sin:
    係数を mod 2 で整理

cos:
    係数を mod 2 で整理

tan:
    係数を mod 1 で整理
```

`tan(qπ)` は、代数化や区間評価より先に極を判定する。`q ≡ 1/2 (mod 1)` であれば `DomainErrorKind::TangentPole` とする。

処理は次の二段階に分ける。

#### 段階A: 特殊角テーブル

```text
sin(π/6) = 1/2
cos(π/4) = sqrt(2)/2
tan(π/3) = sqrt(3)
```

#### 段階B: 円分多項式による一般的な代数化

有理数 (q) に対する (\sin(q\pi)) と (\cos(q\pi)) は代数的数として扱える。`tan(qπ)` は極でないことを先に証明した上で、`sin(qπ) / cos(qπ)` として代数的数へ変換できる。ただし分母が大きいと次数が増大するため、以下で制限する。

```text
max_cyclotomic_order
max_algebraic_degree
max_polynomial_coefficient_bits
```

制限超過時は `sin(qπ)` を形式式のまま保持する。

---

### 4.5 逆三角関数

主値の範囲を仕様で固定する。

```text
asin(x) ∈ [-π/2, π/2]
acos(x) ∈ [0, π]
atan(x) ∈ (-π/2, π/2)
```

特殊値は厳密に認識する。

```text
asin_rad(1/2) = π/6
asin_rad(-1/2) = -π/6
acos_rad(1/2) = π/3
atan_rad(1) = π/4
```

公開結果は `AngleUnit` に従って `from_radians` した厳密値とする。したがってdegree modeでは `asin(1/2) = 30`、radian modeでは `asin(1/2) = π/6` である。同じ三角関数値を持つ角が無数に存在するため、主値範囲を定めずに簡約してはならない。

---

### 4.6 指数・対数

一般には形式式を保持する。

```text
exp(1)
ln(2)
ln(3) / ln(2)
```

厳密に証明できる場合のみ簡約する。

```text
exp(0) = 1
ln(1) = 0
exp(ln(x)) = x      x > 0 が証明済みの場合
ln(exp(x)) = x      実数モードの場合
log(8,2) = 3        明示された底で証明できる場合
log(2^(1/3),2) = 1/3
log(8,sqrt(2)) = 6
```

`ln(xy) = ln(x) + ln(y)` は、実数モードでは (x>0) かつ (y>0) が証明できる場合だけ適用する。

---

### 4.7 累乗の意味論

累乗は指数の種類によって分ける。

公開APIでは次を用いる。初期版のvariantは一つだけだが、request内に明示して意味論の契約にする。

```rust
pub enum PowerSemantics {
    RealPrincipal,
}
```

`RealPrincipal` は、実数領域で主値を返せる場合だけ値を返す。複素数へ入らなければ意味を持てない場合は、形式式へ逃がさず `DomainErrorKind::NonRealPower` とする。ただし、指数が整数または有理数として厳密に認識できる場合は、以下の専用規則を先に適用する。

#### 整数指数

通常の厳密累乗とする。

```text
2^-3 = 1/8
```

#### 有理数指数 (p/q)

(p/q) を既約とし、実数の主根として扱う。

* (q) が偶数なら、底は非負でなければならない。
* (q) が奇数なら、負の底も許可する。
* 0の負指数は定義域エラー。
* `0^0` は既定で不定形エラー。

```text
(-8)^(1/3) = -2
(-8)^(2/3) = 4
(-8)^(1/2) = DomainError
```

#### 一般実数指数

原則として、

```text
x^y = exp(y ln(x))
```

とし、底 (x>0) を要求する。

0を底にする場合は次で固定する。

| 条件 | 結果 |
| ---- | ---- |
| `0^y` かつ `y > 0` が証明済み | `0` |
| `0^0` | `DomainErrorKind::IndeterminateZeroToZero` |
| `0^y` かつ `y < 0` が証明済み | `DomainErrorKind::ZeroToNegativePower` |
| `0^y` で `y` の符号が上限内に証明不能 | `Partial` |

負の底で、整数指数にも有理数指数にも正規化できない一般実数指数は `DomainErrorKind::NonRealPower` とする。例えば `(-1)^(sqrt(2))` は実数モードでは定義域エラーである。

---

## 5. 保証付き近似計算

### 5.1 近似バックエンドをprivate traitで抽象化する

```rust
pub(crate) trait CertifiedFloatBackend {
    type Float;

    fn from_rational_lower(
        value: &Rational,
        precision_bits: u32,
    ) -> Result<Self::Float, BackendError>;

    fn from_rational_upper(
        value: &Rational,
        precision_bits: u32,
    ) -> Result<Self::Float, BackendError>;

    fn next_lower(
        value: &Self::Float,
    ) -> Result<Self::Float, BackendError>;

    fn next_upper(
        value: &Self::Float,
    ) -> Result<Self::Float, BackendError>;
}
```

```rust
pub(crate) struct BackendError {
    kind: BackendErrorKind,
}

pub(crate) enum BackendErrorKind {
    InvalidPrecision,
    NonFiniteInternalValue,
    UnsupportedDirectedRounding,
    BackendInvariantViolation,
}
```

外部crateの浮動小数点型を公開APIへ露出させない。

`astro-float` は、純Rustの任意精度浮動小数点、`no_std`とallocatorの組み合わせ、各種演算・関数・定数の正しい丸めを掲げているため、現時点の第一候補である。([Docs.rs][4])

```toml
astro-float = {
    version = "0.9",
    default-features = false
}
```

ただし、ライブラリの説明をそのまま厳密性の証明として扱ってはならない。特に `Up`、`Down` の丸め仕様が、必要とする方向付き丸めと完全に一致するかをテストとソース監査で確認する必要がある。公開ドキュメント上は複数の丸めモードが定義されている。([Docs.rs][5])

採用条件は次とする。

1. 四則演算と超越関数の正しい丸めを外部oracleと比較する。
2. 正負、subnormal、指数限界、ちょうど中間値を検査する。
3. `next_lower`、`next_upper` の実装を監査する。
4. `NaN`、無限大、内部エラーをcoreの型付きエラーへ変換する。
5. 方向付き丸めが利用できない演算は、最近接丸め結果を隣接値へ1ulp以上外側に拡張する。
6. 保証を構成できない関数については「非保証近似」を返さず、未完了結果とする。

GMP、MPFR、MPCへのFFIを利用する構成はproduction coreには採用しない。`rug`系で使われる基盤はGMP、MPFR、MPCへのRust FFIであるため、「production依存まで純Rust」という条件に合わない。([Docs.rs][6])

---

### 5.2 区間を保証情報の正本とする

```rust
pub struct CertifiedInterval {
    lower: BinaryFloat,
    upper: BinaryFloat,
}
```

不変条件は次である。

```text
lower <= exact value <= upper
```

内部浮動小数点表現はnpm APIへ直接出さず、厳密な二進有理数へ変換する。

```rust
pub struct ExactDyadic {
    coefficient: Integer,
    exponent_two: Integer,
}
```

意味は次である。

```text
coefficient × 2^exponent_two
```

DTOでは両方を10進文字列にする。

```ts
export type ExactDyadicDto = {
    coefficient: SignedDecimalString;
    exponentTwo: SignedDecimalString;
};
```

Ball Arithmeticも将来追加できるが、初期版では区間を正本とする。ボール表示は区間から導出できる。

```text
center = (lower + upper) / 2
radius >= (upper - lower) / 2
```

---

### 5.3 四則演算

加算なら次のように外向きへ丸める。

```text
[a, b] + [c, d]
    ⊆ [roundDown(a + c), roundUp(b + d)]
```

乗算は4通りの積を評価する。

```text
[a, b] × [c, d]
```

```text
min(ac, ad, bc, bd)
max(ac, ad, bc, bd)
```

除算では分母区間が0を含む場合、直ちに定義域エラーとはしない。

1. 厳密な符号・非零証明を試みる。
2. 区間精度を増やす。
3. 真に0なら定義域エラー。
4. 上限まで判定できなければ精度不足の部分結果を返す。

「区間が0を含んだ」と「真の値が0である」は同義ではない。

---

### 5.4 三角関数の区間評価

単純に端点の `sin` や `cos` を計算するだけでは不十分である。区間内部に極値が存在する可能性があるためである。

例えば `sin([a,b])` は次を行う。

1. (\pi) の保証区間を用いて周期を安全に整理する。
2. 区間幅が (2\pi) 以上なら `[-1,1]` を返す。
3. 区間内部に (\pi/2 + k\pi) があるか厳密に判定する。
4. 極値を含む場合は (-1) または (1) を端点候補に加える。
5. 全候補を外向きに丸める。

`tan` では (\pi/2+k\pi) の極を横切るか判定する。判定できない場合は精度を増加させる。

### 5.5 その他の関数の区間評価

Phase 2で扱う初期関数について、区間評価の契約を次に固定する。

| 関数 | 区間評価の契約 |
| ---- | -------------- |
| `sqrt([a,b])` | `b < 0` なら定義域エラー。`a < 0 <= b` の場合は、厳密符号判定または精度増加を試み、真に負を含むなら定義域エラー、判定不能なら `Partial`。有効範囲では単調性により `[sqrt_down(a), sqrt_up(b)]` |
| `exp([a,b])` | 実数全域で単調増加。`[exp_down(a), exp_up(b)]` |
| `ln([a,b])` | `b <= 0` なら定義域エラー。`a <= 0 < b` の場合は符号判定または精度増加。正と証明できた範囲で `[ln_down(a), ln_up(b)]` |
| `asin([a,b])` | `[-1,1]` 外を真に含むなら定義域エラー。定義域境界を含む可能性が未判定なら精度増加または `Partial`。定義域内では単調増加 |
| `acos([a,b])` | `[-1,1]` 外を真に含むなら定義域エラー。定義域内では単調減少し `[acos_down(b), acos_up(a)]` |
| `atan([a,b])` | 実数全域で単調増加 |
| `x^y` | 4.7の実数領域規則を先に適用し、一般実数指数では `exp(y ln(x))` の区間合成を行う。0や負の底は専用規則で処理する |

ここで `*_down` / `*_up` は、関数値の真値を含むよう外向きに丸めた下端・上端を意味する。定義域違反と精度不足は混同しない。区間が危険領域に触れたことだけを理由に定義域エラーへしてはならない。

---

## 6. 指定有効数字への丸め

有効数字 (d) に対して、次の適応的アルゴリズムを使う。

`d` は1以上でなければならない。Rust APIでは `core::num::NonZeroU32` を使い、TypeScript DTOでは正の安全な整数であることをWasm境界で検証する。`0`、負数、非整数、`NaN`、`Infinity` は `InputLimitErrorKind::InvalidSignificantDigits` とする。

丸めは、元の符号付き実数値に対して定義する。指定桁で表せる隣接する二つの10進値を `lower <= x <= upper` としたとき、各 `DecimalRoundingMode` は次の値を選ぶ。

| mode | 契約 |
| ---- | ---- |
| `NearestTiesToEven` | 距離が近い方。ちょうど中間なら最下位桁が偶数の方 |
| `NearestTiesAwayFromZero` | 距離が近い方。ちょうど中間なら絶対値が大きい方 |
| `TowardPositiveInfinity` | `x` 以上の最小候補 |
| `TowardNegativeInfinity` | `x` 以下の最大候補 |
| `TowardZero` | 0方向の候補。正なら下側、負なら上側 |
| `AwayFromZero` | 0から遠い候補。正なら上側、負なら下側 |

2有効数字での例は次である。

| 入力 | even | ties-away | +∞ | -∞ | zero | away |
| ---- | ---- | --------- | -- | -- | ---- | ---- |
| `1.25` | `1.2` | `1.3` | `1.3` | `1.2` | `1.2` | `1.3` |
| `-1.25` | `-1.2` | `-1.3` | `-1.2` | `-1.3` | `-1.2` | `-1.3` |

### 6.1 初期精度

```text
initial_bits
    = ceil((d + guard_decimal_digits) × log2(10))
```

例えばguardを16桁程度とし、以後は決定的な規則で増加させる。

```text
p_next = max(p + 64, ceil(3p / 2))
```

### 6.2 10進指数を確定する

値が0でないことを証明した後、

```text
10^n <= |x| < 10^(n+1)
```

を満たす整数 (n) を保証区間と厳密比較で求める。

### 6.3 丸め対象の整数を確定する

```text
scaled = |x| × 10^(d - 1 - n)
```

最近接・偶数丸めなら、区間全体が同一の丸めセルへ入るかを判定する。

区間両端が同じ整数へ丸められるなら、その桁は確定している。

丸め境界ちょうどに値がある可能性がある場合、以下を優先する。

1. 有理数なら厳密比較。
2. 代数的数なら多項式符号判定。
3. 形式式なら区間精度を増加。
4. 上限まで決まらなければ未確定。

### 6.4 桁上がり

例えば3桁指定で結果が内部的に、

```text
9.999... × 10^4
```

から

```text
10.0 × 10^4
```

へ丸められた場合、正規化して次を返す。

```text
1.00 × 10^5
```

### 6.5 0の表示

0は通常の科学表記正規化条件を満たさないため、特別規則を定める。

3桁指定なら、

```text
0.00 × 10^0
```

とする。

### 6.6 精度上限到達

指定桁を一意に確定できなければ、推測した小数を返さない。

```rust
pub enum CalculationOutcome {
    Complete(Calculation),
    Partial {
        calculation: Calculation,
        reason: IncompleteReason,
        certified_enclosure: CertifiedIntervalPresentation,
    },
}

pub enum IncompleteReason {
    PrecisionLimit {
        requested_digits: u32,
        confirmed_digits: u32,
    },
    ComputationLimit {
        kind: ComputationLimitKind,
    },
}
```

`Complete` は、requestで要求されたすべての出力が確定したことを意味する。`Partial` は、少なくとも一つの要求出力または内部判定が計算量上限内に完了しなかったことを意味する。

`Partial` でも、確定済みの厳密式と現在の保証区間を含める。これは `EnclosureOutputRequest::Omit` とは独立した安全性情報であり、表示用のenclosure欄を省略していても、`Partial.certified_enclosure` には機械可読な保証区間を入れる。指定桁を一意に確定できなかった場合、推測した `significand`、`exponent_ten`、表示木は返さない。

---

## 7. 簡約エンジン

### 7.1 証明状態を三値で扱う

```rust
pub enum Truth {
    Proven,
    Disproven,
    Unknown,
}
```

符号についても、単なる `bool` ではなく次を使う。

```rust
pub enum SignKnowledge {
    Negative,
    Zero,
    Positive,
    NonNegative,
    NonPositive,
    NonZero,
    Unknown,
}
```

### 7.2 危険な簡約例

```text
sqrt(x²) → |x|
```

は常に正しいが、

```text
sqrt(x²) → x
```

は (x \ge 0) が証明できる場合だけ正しい。

同様に、

```text
x / x → 1
```

は (x \ne 0) が証明できる場合だけ適用する。

```text
(x^a)^b → x^(ab)
```

も、実数上では底や指数によって不正となるため、無条件に適用しない。

---

### 7.3 簡約段階

順序を固定する。

```text
1. リテラル正規化
2. 有理数演算
3. Add / Multiply の平坦化
4. 項の決定的ソート
5. 有理係数の統合
6. 根号簡約
7. 代数的数演算
8. π有理数倍の認識
9. 既知関数値
10. 条件付き恒等式
11. 表示候補生成
```

最初から無制限のe-graph型探索は採用しない。探索順、証明条件、計算量制限、結果の決定性を管理しやすい、段階的なbottom-up簡約を採用する。

---

### 7.4 計算用正規形と表示形式を分ける

代数的数の最小多項式は計算には適していても、人間向け表示に適するとは限らない。

例えば (\sqrt{2}) は内部的には、

```text
Root(x² - 2, positive)
```

であっても、表示では、

```text
sqrt(2)
```

を優先する。

表示候補の優先順位は概ね次とする。

```text
整数
既約分数
有限小数
帯分数
πの有理数倍
単純根号
既知の代数式
簡約済み形式式
代数的根の記述
```

候補選択には決定的なコスト関数を用いる。

```text
ノード数
演算子深さ
分子・分母の桁数
根号の数
関数呼び出し数
```

---

## 8. 表示用中間表現

coreからHTML文字列を返さない。UIに依存しない表示木を返す。

```rust
pub enum PresentationNode {
    Text(String),
    Row(Vec<PresentationNode>),
    Fraction {
        numerator: Box<PresentationNode>,
        denominator: Box<PresentationNode>,
    },
    Superscript {
        base: Box<PresentationNode>,
        exponent: Box<PresentationNode>,
    },
    Subscript {
        base: Box<PresentationNode>,
        subscript: Box<PresentationNode>,
    },
    Radical {
        index: RadicalIndex,
        radicand: Box<PresentationNode>,
    },
    Function {
        name: FunctionName,
        argument: Box<PresentationNode>,
    },
    Parenthesized(Box<PresentationNode>),
}

pub enum RadicalIndex {
    Square,
    Nth(PositiveInteger),
}

pub enum FunctionName {
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Sqrt,
    Exp,
    Log,
}
```

これにより、利用側は同じ結果を次の形式へ変換できる。

* plain text
* MathML
* LaTeX
* 独自DOM
* Canvas描画
* 音声読み上げ

関係記号も型で表現する。

```rust
pub enum ResultRelation {
    ExactEqual,
    ApproximatelyEqual,
    ElementOf,
}
```

表示は次のようになる。

```text
= 1/2
≈ 5.000000000 × 10^-1
∈ [0.4999999999..., 0.5000000001...]
```

科学表記は要件に従い、有限小数と一致していても原則 `≈` を使用する。厳密有限小数は厳密表示欄で `=` として表示する。

---

## 9. Rust公開API

### 9.1 基本API

```rust
pub fn calculate(
    source: &str,
    request: &CalculationRequest,
    context: &mut EvaluationContext,
) -> Result<CalculationOutcome, CalculatorError>;
```

パーサーと評価器を個別利用できるAPIも持たせる。

```rust
pub fn parse(
    source: &str,
    settings: &ParseSettings,
) -> Result<ParsedExpression, ParseError>;

pub fn evaluate(
    expression: &ParsedExpression,
    request: &EvaluationRequest,
    context: &mut EvaluationContext,
) -> Result<EvaluationOutcome, EvaluationError>;

pub fn present(
    evaluation: &EvaluationOutcome,
    request: &PresentationRequest,
) -> Result<Calculation, PresentationError>;
```

`ParsedExpression` の内部ASTは公開しない。公開すると構文木構造が公開契約になり、将来の正規化変更が困難になるためである。

`ParsedExpression` と `EvaluationContext` は公開型だが内部fieldを公開しないopaque typeとする。

```rust
pub struct ParsedExpression {
    _private: (),
}

pub struct EvaluationContext {
    _private: (),
}

pub struct EvaluationOutcome {
    pub value: EvaluatedValue,
    pub metadata: EvaluationMetadata,
}

pub struct EvaluationMetadata {
    pub methods: Vec<MethodTag>,
    pub internal_precision_bits: u32,
    pub refinement_rounds: u32,
}

pub enum EvaluationError {
    Domain(DomainError),
    InputLimit(InputLimitError),
    ComputationLimit(ComputationLimitError),
    UnsupportedFeature(UnsupportedFeatureError),
    InternalInvariant(InternalInvariantError),
}

pub enum PresentationError {
    InputLimit(InputLimitError),
    ComputationLimit(ComputationLimitError),
    InternalInvariant(InternalInvariantError),
}
```

`EvaluationContext` が保持してよいものは、intern table、memoization cache、一時buffer、統計情報に限る。cache hitの有無によって、計算結果、エラー種別、`Partial` 判定、`logical_work_units` の課金結果が変わってはならない。cacheを使った場合でも、計算量制限の判定は「cache missとして実行した場合に必要な論理作業量」に基づいて行う。

---

### 9.2 request

```rust
pub struct CalculationRequest {
    pub parse: ParseSettings,
    pub semantics: SemanticSettings,
    pub exact_output: ExactOutputRequest,
    pub scientific_output: ScientificOutputRequest,
    pub enclosure_output: EnclosureOutputRequest,
    pub limits: ResourceLimitRequest,
}

pub struct ParseSettings {
    pub grammar: GrammarProfile,
    pub implicit_multiplication: ImplicitMultiplicationPolicy,
    pub unicode_aliases: UnicodeAliasPolicy,
    pub percent: PercentParsePolicy,
}

pub enum GrammarProfile {
    Default,
}

pub enum ImplicitMultiplicationPolicy {
    Enabled,
    Disabled,
}

pub enum UnicodeAliasPolicy {
    MathematicalAliases,
    AsciiOnly,
}

pub enum PercentParsePolicy {
    PostfixPercent,
    RejectPercent,
}

pub struct SemanticSettings {
    pub domain: EvaluationDomain,
    pub angle_unit: AngleUnit,
    pub power_semantics: PowerSemantics,
}

pub enum ExactOutputRequest {
    Omit,
    Include {
        format: ExactFormatPreference,
    },
}

pub enum ScientificOutputRequest {
    Omit,
    Include {
        significant_digits: core::num::NonZeroU32,
        rounding_mode: DecimalRoundingMode,
    },
}

pub enum EnclosureOutputRequest {
    Omit,
    Include {
        format: EnclosureFormat,
    },
}

pub enum ResourceLimitRequest {
    Default,
    Custom(ResourceLimits),
}

pub struct EvaluationRequest {
    pub semantics: SemanticSettings,
    pub limits: ResourceLimitRequest,
}

pub struct PresentationRequest {
    pub exact_output: ExactOutputRequest,
    pub scientific_output: ScientificOutputRequest,
    pub enclosure_output: EnclosureOutputRequest,
}
```

`calculate(source, request, context)` は、同じ `request.parse` で `parse` し、同じ `request.semantics` と `request.limits` で `evaluate` し、同じ出力requestで `present` する合成APIである。個別APIを使う場合でも、合成APIと同じDTOを生成しなければならない。

初期版のformat指定は次で固定する。

```rust
pub enum ExactFormatPreference {
    Auto,
    Rational,
    FiniteDecimal,
    MixedFraction,
    Symbolic,
}

pub enum EnclosureFormat {
    ExactDyadic,
}
```

### 9.3 丸めモード

```rust
pub enum DecimalRoundingMode {
    NearestTiesToEven,
    NearestTiesAwayFromZero,
    TowardPositiveInfinity,
    TowardNegativeInfinity,
    TowardZero,
    AwayFromZero,
}
```

既定値は `NearestTiesToEven` とする。

---

### 9.4 結果型

```rust
pub struct Calculation {
    pub exact: ExactOutput,
    pub scientific: ScientificOutput,
    pub enclosure: EnclosureOutput,
    pub metadata: CalculationMetadata,
}

pub enum ExactOutput {
    Omitted,
    Included(ExactPresentation),
}

pub enum ScientificOutput {
    Omitted,
    Included(ScientificPresentation),
    Unavailable(UnavailableScientificOutput),
}

pub enum EnclosureOutput {
    Omitted,
    Included(CertifiedIntervalPresentation),
}
```

```rust
pub struct ExactPresentation {
    pub relation: ResultRelation,
    pub representation: ExactRepresentationKind,
    pub presentation: PresentationNode,
    pub plain_text: String,
}

pub struct ScientificPresentation {
    pub relation: ResultRelation,
    pub significand: String,
    pub exponent_ten: String,
    pub requested_significant_digits: u32,
    pub confirmed_significant_digits: u32,
    pub rounding_mode: DecimalRoundingMode,
    pub presentation: PresentationNode,
}

pub struct UnavailableScientificOutput {
    pub requested_significant_digits: core::num::NonZeroU32,
    pub confirmed_significant_digits: u32,
    pub rounding_mode: DecimalRoundingMode,
    pub reason: IncompleteReason,
}

pub struct CertifiedIntervalPresentation {
    pub relation: ResultRelation,
    pub lower: ExactDyadic,
    pub upper: ExactDyadic,
    pub format: EnclosureFormat,
    pub presentation: PresentationNode,
}
```

10進指数は極端に大きくなり得るため文字列で保持する。`ScientificOutput::Unavailable` は `CalculationOutcome::Partial` の内部でだけ使用する。`significand`、`exponent_ten`、表示用 `PresentationNode` を持たないため、推測した小数表示が外部へ漏れない。

---

### 9.5 metadata

```rust
pub struct CalculationMetadata {
    pub exact_representation: ExactRepresentationKind,
    pub simplification_status: SimplificationStatus,
    pub semantic_settings: SemanticSettings,
    pub methods: Vec<MethodTag>,
    pub internal_precision_bits: u32,
    pub refinement_rounds: u32,
    pub confirmed_significant_digits: u32,
    pub assurance: AssuranceLevel,
    pub protocol_version: ProtocolVersion,
}
```

```rust
pub enum ExactRepresentationKind {
    Integer,
    Rational,
    FiniteDecimal,
    RationalPiMultiple,
    Radical,
    RealAlgebraic,
    GeneralSymbolic,
}

pub enum SimplificationStatus {
    FullySimplifiedWithinLimits,
    PartiallySimplified {
        reason: IncompleteReason,
    },
}

pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

pub enum MethodTag {
    RationalReduction,
    RadicalExtraction,
    SpecialAngle,
    CyclotomicReduction,
    AlgebraicMinimalPolynomial,
    AlgebraicRootIsolation,
    SymbolicRetention,
    CertifiedIntervalEvaluation,
    AdaptivePrecisionRefinement,
}
```

`methods` は不安定なデバッグログではなく、意味的に安定した分類だけを返す。

---

### 9.6 protocol versionとsnapshot

`ProtocolVersion` は現行DTO surfaceを識別するためのversionであり、npm packageのsemverとは別に扱う。試作段階では後方互換性を保証しないが、DTO変更を見落とさないようsnapshotを更新する。

```text
major:
    既存DTOの意味変更、必須field削除、既存codeの意味変更など、
    surfaceの大きな変更で増加する。

minor:
    optional field追加、新しいenum/code追加、新しいMethodTag追加など、
    surfaceの追加で増加する。
```

初期公開時のprotocol versionは `1.0` とし、現行の公開DTO contractは `1.1` とする。Rust公開enumのうち、利用者が網羅matchし得るものには、必要に応じて `#[non_exhaustive]` を付ける。ただし、計算意味論に関わる `DomainErrorKind`、`DecimalRoundingMode`、`PowerSemantics` は追加時にminor以上のversion更新を必要とする。

TypeScript facadeは、未知の `tag` や未知の `code` を受け取った場合、握りつぶさず `unsupportedProtocol` エラーへ変換する。未知のDTOを誤った成功値として扱ってはならない。

---

### 9.7 「証明付き」の表現

APIでの保証レベルを明示する。

```rust
pub enum AssuranceLevel {
    Exact,
    CertifiedEnclosure,
}
```

ここで `CertifiedEnclosure` は、実装が数学的に外包を保証するという意味である。

独立した別実装で検証可能な「形式的証明オブジェクト」を返すわけではない。その機能まで要求する場合は、将来的に次を別途設計する。

```rust
pub enum PortableCertificate {
    PolynomialRootIsolation(PolynomialRootCertificate),
    SeriesRemainderBound(SeriesCertificate),
}

pub struct PolynomialRootCertificate {
    _private: (),
}

pub struct SeriesCertificate {
    _private: (),
}
```

検証器を持たない段階で「machine-verifiable proof」とは表記しない。

---

## 10. エラー設計

エラー値と表示メッセージを分ける。

```rust
pub enum CalculatorError {
    Parse(ParseError),
    Domain(DomainError),
    InputLimit(InputLimitError),
    ComputationLimit(ComputationLimitError),
    UnsupportedFeature(UnsupportedFeatureError),
    InternalInvariant(InternalInvariantError),
}
```

```rust
pub struct DomainError {
    pub kind: DomainErrorKind,
    pub span: Option<ByteSpan>,
}

pub enum DomainErrorKind {
    DivisionByZero,
    LogarithmOfNonPositive,
    EvenRootOfNegative,
    InverseTrigonometricOutOfRange,
    TangentPole,
    ZeroToNegativePower,
    IndeterminateZeroToZero,
    NonRealPower,
}
```

```rust
pub enum ParseErrorKind {
    UnexpectedToken,
    UnexpectedEnd,
    UnknownIdentifier,
    InvalidNumberLiteral,
    MissingFunctionParenthesis,
    ImplicitMultiplicationDisabled,
    PercentRejected,
}

pub struct ParseError {
    pub kind: ParseErrorKind,
    pub span: ByteSpan,
    pub expected: Vec<ExpectedToken>,
}

pub struct ExpectedToken {
    pub kind: ExpectedTokenKind,
}

pub enum ExpectedTokenKind {
    Number,
    Identifier,
    Operator,
    OpenParenthesis,
    CloseParenthesis,
    EndOfInput,
}

pub struct ByteSpan {
    pub start: u32,
    pub end: u32,
}
```

```rust
pub struct InputLimitError {
    pub kind: InputLimitErrorKind,
}

pub enum InputLimitErrorKind {
    InputTooLong,
    SourceAstTooDeep,
    SourceAstTooLarge,
    ExpressionTooLarge,
    IntegerTooLarge,
    OutputTooLarge,
    InvalidSignificantDigits,
    InvalidResourceLimit,
}

pub struct ComputationLimitError {
    pub kind: ComputationLimitKind,
}

pub enum ComputationLimitKind {
    AlgebraicDegree,
    PolynomialCoefficientBits,
    ResultantDegree,
    FactorizationWork,
    RootIsolationSteps,
    RewriteSteps,
    PrecisionBits,
    RefinementRounds,
    LogicalWorkUnits,
    PresentationNodes,
}

pub struct UnsupportedFeatureError {
    pub feature: UnsupportedFeature,
}

pub enum UnsupportedFeature {
    ComplexDomain,
    PortableProofCertificate,
}

pub struct InternalInvariantError {
    pub code: InternalInvariantCode,
}

pub enum InternalInvariantCode {
    NonCanonicalRational,
    InvalidAlgebraicIsolation,
    InvalidCertifiedInterval,
    NonDeterministicCacheAccounting,
}
```

coreではUTF-8のbyte offsetを正本とする。WasmアダプターでDOM向けのUTF-16 code unit offsetを併記する。

通常の計算量制限超過は、厳密式や保証区間を返せるなら `Partial` とする。入力長超過やAST深度超過のように、評価自体を安全に開始できない場合はエラーとする。

Wasm DTOでは、errorを次のtagged unionへ変換する。

```ts
export type CalculatorErrorDto =
    | {
        tag: "parse";
        code: ParseErrorCode;
        spanUtf8: TextSpanDto;
        spanUtf16: TextSpanDto;
        expected: ExpectedTokenDto[];
    }
    | {
        tag: "domain";
        code: DomainErrorCode;
        spanUtf8: OptionalTextSpanDto;
        spanUtf16: OptionalTextSpanDto;
    }
    | {
        tag: "inputLimit";
        code: InputLimitErrorCode;
    }
    | {
        tag: "computationLimit";
        code: ComputationLimitCode;
    }
    | {
        tag: "unsupportedFeature";
        code: UnsupportedFeatureCode;
    }
    | {
        tag: "internalInvariant";
        code: InternalInvariantCode;
    }
    | {
        tag: "unsupportedProtocol";
        code: UnsupportedProtocolCode;
    };
```

`code` は英小文字camelCaseの安定識別子とし、localized messageをDTOへ含めない。表示文言、言語、色、フォントは利用側のpresentation層で決める。

---

## 11. 決定的な計算量制限

壁時計時間によるtimeoutはcoreでは使用しない。同じマシンでも負荷状況によって結果が変わるためである。

```rust
pub struct ResourceLimits {
    pub max_input_bytes: u32,
    pub max_source_ast_nodes: u32,
    pub max_source_depth: u32,
    pub max_expression_nodes: u32,
    pub max_integer_bits: u32,
    pub max_algebraic_degree: u32,
    pub max_polynomial_coefficient_bits: u32,
    pub max_resultant_degree: u32,
    pub max_factorization_work: u32,
    pub max_root_isolation_steps: u32,
    pub max_rewrite_steps: u32,
    pub max_precision_bits: u32,
    pub max_refinement_rounds: u32,
    pub max_logical_work_units: u64,
    pub max_presentation_nodes: u32,
    pub max_output_bytes: u32,
}
```

Rust APIでは `max_logical_work_units` は `u64` とする。TypeScript DTOでは、この値だけは安全整数範囲を超え得るため、canonical unsigned decimal stringとして渡す。

`logical_work_units` は実行時間ではなく、アルゴリズム上の操作へ決定的に課金する。

例えば、

```text
BigInt乗算:
    オペランドbit長に応じて課金

resultant:
    入力次数と係数bit長に応じて課金

rewrite:
    規則適用試行ごとに課金
```

キャッシュヒットによって計算結果や制限判定が変わってはならない。共有キャッシュを使う場合でも、logical costはキャッシュの有無に依存させない。

---

## 12. npmおよびWasm境界

### 12.1 生のwasm-bindgen出力を公開APIにしない

構造は次のようにする。

```text
Rust core types
    ↓ 明示的変換
Wasm DTO
    ↓ serde-wasm-bindgen
Generated JS binding
    ↓ private
Hand-written TypeScript facade
    ↓ public npm API
```

`wasm-bindgen` の `--target web` はブラウザーから直接読み込めるES Moduleを生成でき、npm向け生成には `wasm-pack` を利用できる。([wasm-bindgen.github.io][7])

生成された低水準関数名や `JsValue` を利用者に直接公開しない。公開TypeScript APIを手書きのfacadeで固定する。

---

### 12.2 DTO

`serde-wasm-bindgen` はRust型とJavaScriptのnative object間の変換に利用できる。ただし `Option::None` は既定で `undefined` または `null` へ変換され得るため、公開DTOでは `Option` を直接使用しない。明示的なtagged unionへ変換する。([Docs.rs][8])

```ts
export type ApiResult<T> =
    | {
        tag: "ok";
        value: T;
    }
    | {
        tag: "error";
        error: CalculatorError;
    };

export type CalculationOutcome =
    | {
        tag: "complete";
        calculation: Calculation;
    }
    | {
        tag: "partial";
        calculation: Calculation;
        reason: IncompleteReason;
        certifiedEnclosure: CertifiedIntervalPresentation;
    };
```

`Partial` DTOにも `certifiedEnclosure` を含める。これは `enclosureOutput` の表示要求とは独立した安全性情報である。

出力指定も `undefined` を用いない。

```ts
export type ExactOutputRequest =
    | {
        tag: "omit";
    }
    | {
        tag: "include";
        format: ExactFormatPreference;
    };
```

resource limit指定もtagged unionとする。

```ts
export type ResourceLimitRequest =
    | {
        tag: "default";
    }
    | {
        tag: "custom";
        value: ResourceLimitsDto;
    };

export type ResourceLimitsDto = {
    maxInputBytes: number;
    maxSourceAstNodes: number;
    maxSourceDepth: number;
    maxExpressionNodes: number;
    maxIntegerBits: number;
    maxAlgebraicDegree: number;
    maxPolynomialCoefficientBits: number;
    maxResultantDegree: number;
    maxFactorizationWork: number;
    maxRootIsolationSteps: number;
    maxRewriteSteps: number;
    maxPrecisionBits: number;
    maxRefinementRounds: number;
    maxLogicalWorkUnits: UnsignedDecimalString;
    maxPresentationNodes: number;
    maxOutputBytes: number;
};
```

span DTOは次で固定する。

```ts
export type TextSpanDto = {
    start: number;
    end: number;
};

export type OptionalTextSpanDto =
    | {
        tag: "none";
    }
    | {
        tag: "some";
        value: TextSpanDto;
    };
```

`number` で受ける設定値は、Wasm境界で `Number.isSafeInteger(value)`、非負、上限内であることを検証する。`NaN`、`Infinity`、小数、負数、`-0`、文字列化された数値の暗黙受理は禁止する。

---

### 12.3 JavaScriptの`number`を使う範囲

次は `number` を許可する。

* 有効数字数
* ASTノード数上限
* 最大反復回数
* UI上のindex
* `u32` に制限された設定値

次は必ず文字列または型付き構造とする。

* 任意精度整数
* 有理数の分子・分母
* 高精度小数
* 二進指数
* 10進指数
* 区間端点
* 多項式係数

数値文字列は次のASCII grammarを正本とする。

```text
UnsignedDecimalString = "0" | NonZeroDigit Digit*
SignedDecimalString   = "-"? UnsignedDecimalString
NonZeroDigit          = "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
Digit                 = "0" | NonZeroDigit
```

canonical formでは、`0` 以外の先頭ゼロを禁止し、負のゼロ `-0` を禁止する。桁区切り、空白、locale依存文字、指数表記、`NaN`、`Infinity` は禁止する。有理数DTOの分母は `UnsignedDecimalString` かつ `0` ではない。

```ts
export type SignedDecimalString = string;
export type UnsignedDecimalString = string;

export type RationalDto = {
    numerator: SignedDecimalString;
    denominator: UnsignedDecimalString;
};

export type ExactDyadicDto = {
    coefficient: SignedDecimalString;
    exponentTwo: SignedDecimalString;
};
```

---

### 12.4 TypeScript型の生成

`serde-wasm-bindgen` 自体はTypeScript型を自動生成しないため、DTO定義からの型生成処理を `xtask` に置く。公式文書も、TypeScript生成は別途 `tsify` などとの統合が必要としている。([Docs.rs][8])

DTOのsource of truthは `crates/calculator-wasm/src/dto.rs` とする。DTO型は原則として次のserde属性に従う。

```rust
#[serde(rename_all = "camelCase")]
#[serde(tag = "tag")]
```

Rust enumの内部表現をそのまま公開せず、Wasm境界でDTOへ明示変換する。生成されたTypeScript型は `packages/calculator/src/generated/dto.ts` へ出力し、手編集を禁止する。手書きfacadeは必要な型を `packages/calculator/src/index.ts` から再exportする。

CIでは次を検査する。

```text
cargo xtask generate-types
git diff --exit-code
pnpm --dir packages/calculator tsc --noEmit
```

Rust DTOを変更して `.d.ts` の再生成を忘れた場合、CIを失敗させる。

---

### 12.5 npm公開API

パッケージ名の例を次とする。

```text
@bem130/exact-calculator
```

利用例は以下である。

```ts
import {
    createCalculator,
    type CalculationRequest,
} from "@bem130/exact-calculator";

const calculator = await createCalculator();

const request: CalculationRequest = {
    parse: {
        grammar: "default",
        implicitMultiplication: "enabled",
        unicodeAliases: "mathematicalAliases",
        percent: "postfixPercent",
    },
    semantics: {
        domain: "real",
        angleUnit: "radian",
        powerSemantics: "realPrincipal",
    },
    exactOutput: {
        tag: "include",
        format: "auto",
    },
    scientificOutput: {
        tag: "include",
        significantDigits: 50,
        roundingMode: "nearestTiesToEven",
    },
    enclosureOutput: {
        tag: "include",
        format: "exactDyadic",
    },
    limits: {
        tag: "default",
    },
};

const outcome = calculator.calculate(
    "sin(pi / 6) + sqrt(2)",
    request,
);
```

初期化だけを非同期とし、直接版の計算自体は同期にできる。

```ts
export interface Calculator {
    calculate(
        source: string,
        request: CalculationRequest,
    ): ApiResult<CalculationOutcome>;
}
```

長時間計算用にはworker版を別exportする。

```ts
import {
    createWorkerCalculator,
} from "@bem130/exact-calculator/worker";

const calculator = await createWorkerCalculator();

const outcome = await calculator.calculate(
    "sin(1) + ln(2)",
    request,
    {
        signal: {
            tag: "abortSignal",
            signal: abortController.signal,
        },
    },
);
```

```ts
export interface WorkerCalculator {
    calculate(
        source: string,
        request: CalculationRequest,
        options: WorkerCalculationOptions,
    ): Promise<ApiResult<CalculationOutcome>>;

    terminate(): void;
}

export type WorkerCalculationOptions = {
    signal: AbortSignalOption;
};

export type AbortSignalOption =
    | {
        tag: "none";
    }
    | {
        tag: "abortSignal";
        signal: AbortSignal;
    };
```

サンプルUIが必要とする補助APIも、private Wasm関数ではなくnpm公開APIとして提供する。

```ts
export interface CalculatorSession {
    dispatch(action: InputActionDto): SessionDispatchResult;
    applyResult(result: ApiResult<CalculationOutcome>): SessionStateDto;
    getState(): SessionStateDto;
}

export type SessionDispatchResult =
    | {
        tag: "state";
        state: SessionStateDto;
    }
    | {
        tag: "calculate";
        state: SessionStateDto;
        source: string;
        request: CalculationRequest;
    };

export function createSession(policy: InputPolicyDto): CalculatorSession;

export function renderPlainText(node: PresentationNodeDto): string;
export function renderMathMl(node: PresentationNodeDto): string;
```

`renderMathMl` は文字列を返すが、DOMへ挿入する責務は持たない。DOM sanitization、clipboard、ARIA属性はsample UI側の責務とする。

公開facadeで使う補助DTOは次の形を正本とする。詳細なfieldはgenerated DTOから再exportするが、`tag` と `code` の値はこの文書のRust enumとcamelCaseで対応する。

```ts
export type ParseErrorCode =
    | "unexpectedToken"
    | "unexpectedEnd"
    | "unknownIdentifier"
    | "invalidNumberLiteral"
    | "missingFunctionParenthesis"
    | "implicitMultiplicationDisabled"
    | "percentRejected";

export type DomainErrorCode =
    | "divisionByZero"
    | "logarithmOfNonPositive"
    | "evenRootOfNegative"
    | "inverseTrigonometricOutOfRange"
    | "tangentPole"
    | "zeroToNegativePower"
    | "indeterminateZeroToZero"
    | "nonRealPower";

export type InputLimitErrorCode =
    | "inputTooLong"
    | "sourceAstTooDeep"
    | "sourceAstTooLarge"
    | "expressionTooLarge"
    | "integerTooLarge"
    | "outputTooLarge"
    | "invalidSignificantDigits"
    | "invalidResourceLimit";

export type ComputationLimitCode =
    | "algebraicDegree"
    | "polynomialCoefficientBits"
    | "resultantDegree"
    | "factorizationWork"
    | "rootIsolationSteps"
    | "rewriteSteps"
    | "precisionBits"
    | "refinementRounds"
    | "logicalWorkUnits"
    | "presentationNodes";

export type UnsupportedFeatureCode =
    | "complexDomain"
    | "portableProofCertificate";

export type InternalInvariantCode =
    | "nonCanonicalRational"
    | "invalidAlgebraicIsolation"
    | "invalidCertifiedInterval"
    | "nonDeterministicCacheAccounting";

export type UnsupportedProtocolCode =
    | "unknownTag"
    | "unknownCode"
    | "unsupportedMajorVersion";

export type ExpectedTokenDto = {
    kind: ExpectedTokenKindDto;
};

export type ExpectedTokenKindDto =
    | "number"
    | "identifier"
    | "operator"
    | "openParenthesis"
    | "closeParenthesis"
    | "endOfInput";

export type PresentationNodeDto =
    | { tag: "text"; text: string }
    | { tag: "row"; children: PresentationNodeDto[] }
    | { tag: "fraction"; numerator: PresentationNodeDto; denominator: PresentationNodeDto }
    | { tag: "superscript"; base: PresentationNodeDto; exponent: PresentationNodeDto }
    | { tag: "radical"; index: RadicalIndexDto; radicand: PresentationNodeDto }
    | { tag: "function"; name: FunctionNameDto; argument: PresentationNodeDto }
    | { tag: "parenthesized"; value: PresentationNodeDto };

export type InputActionDto =
    | { tag: "digit"; value: number }
    | { tag: "decimalPoint" }
    | { tag: "constant"; value: ConstantDto }
    | { tag: "function"; value: FunctionDto }
    | { tag: "binaryOperator"; value: BinaryOperatorDto }
    | { tag: "percent" }
    | { tag: "openParenthesis" }
    | { tag: "closeParenthesis" }
    | { tag: "deleteBackward" }
    | { tag: "clearEntry" }
    | { tag: "clearAll" }
    | { tag: "memoryClear" }
    | { tag: "memoryRecall" }
    | { tag: "memoryAdd" }
    | { tag: "memorySubtract" }
    | { tag: "evaluate" };

export type InputPolicyDto = {
    calculationRequest: CalculationRequest;
    percentPolicy: PercentPolicyDto;
};

export type PercentPolicyDto =
    | "expressionPercent"
    | "calculatorPercent";

export type SessionStateDto = {
    source: string;
    cursorUtf16: number;
    selectionUtf16: OptionalTextSpanDto;
    hasAns: boolean;
    hasMemory: boolean;
    display: SessionDisplayDto;
};

export type RadicalIndexDto =
    | { tag: "square" }
    | { tag: "nth"; value: UnsignedDecimalString };

export type FunctionNameDto =
    | "sin"
    | "cos"
    | "tan"
    | "asin"
    | "acos"
    | "atan"
    | "sqrt"
    | "exp"
    | "log";

export type ConstantDto =
    | "pi"
    | "e"
    | "ans"
    | "memory";

export type FunctionDto = FunctionNameDto;

export type BinaryOperatorDto =
    | "add"
    | "subtract"
    | "multiply"
    | "divide"
    | "power";

export type SessionDisplayDto =
    | { tag: "editing" }
    | { tag: "result"; calculation: Calculation }
    | { tag: "error"; error: CalculatorErrorDto }
    | { tag: "calculating" };
```

これらのDTOはgenerated DTOで定義し、いずれも `null` / `undefined` を使わないtagged unionまたは文字列unionとする。

---

## 13. 人間のボタン操作向けsession

数式文字列APIを主APIとし、その上に任意利用のheadless sessionを置く。

```rust
pub enum InputAction {
    Digit(u8),
    DecimalPoint,
    Constant(Constant),
    Function(Function),
    BinaryOperator(BinaryOperator),
    Percent,
    OpenParenthesis,
    CloseParenthesis,
    DeleteBackward,
    ClearEntry,
    ClearAll,
    MemoryClear,
    MemoryRecall,
    MemoryAdd,
    MemorySubtract,
    Evaluate,
}
```

```rust
pub struct InputState {
    _private: (),
}

pub struct InputError {
    pub kind: InputErrorKind,
}

pub enum InputErrorKind {
    InvalidDigit,
    InvalidCursor,
    SelectionOutOfBounds,
    ActionNotAllowedAfterError,
    MemoryEmpty,
}

pub struct SessionReduction {
    pub state: InputState,
    pub command: SessionCommand,
}

pub enum SessionCommand {
    None,
    Calculate {
        source: String,
        request: CalculationRequest,
    },
}

pub fn reduce_input(
    state: &InputState,
    action: InputAction,
    policy: &InputPolicy,
) -> Result<SessionReduction, InputError>;

pub fn apply_calculation_result(
    state: &InputState,
    result: Result<CalculationOutcome, CalculatorError>,
) -> InputState;
```

`reduce_input` と `apply_calculation_result` は純粋関数とする。`Evaluate` は計算を実行せず、`SessionCommand::Calculate` を返す。CLI、Web worker、UI event handlerなどの表層がそのcommandを実行し、結果を `apply_calculation_result` へ渡す。

sessionが扱うものは以下である。

* カーソル位置
* 選択範囲
* 括弧補完
* 関数ボタン挿入
* `Ans`
* メモリー
* 文脈依存パーセント
* 入力履歴

一方、以下はsessionへ置かない。

* 数式の厳密意味論
* 有理数演算
* 三角関数計算
* 結果丸め
* 簡約規則

独自UIの利用者はsessionを無視し、直接 `calculate()` を使える。

`InputPolicy` には少なくとも次を含める。

```rust
pub struct InputPolicy {
    pub calculation_request: CalculationRequest,
    pub percent_policy: PercentPolicy,
}

pub enum PercentPolicy {
    ExpressionPercent,
    CalculatorPercent,
}
```

`ExpressionPercent` では、`Percent` actionは現在の直前項へ後置 `%` を挿入し、文字列APIと同じく `x / 100` としてparseされる。`CalculatorPercent` では、session reducerが直前の二項演算文脈を見て、計算機型のpercentへloweringする。

| 入力列 | ExpressionPercent | CalculatorPercent |
| ------ | ----------------- | ----------------- |
| `100 + 10 % Evaluate` | `100 + (10 / 100)` | `100 + (100 × 10 / 100)` |
| `100 - 10 % Evaluate` | `100 - (10 / 100)` | `100 - (100 × 10 / 100)` |
| `100 * 10 % Evaluate` | `100 × (10 / 100)` | `100 × (10 / 100)` |
| `100 / 10 % Evaluate` | `100 / (10 / 100)` | `100 / (10 / 100)` |
| `50 % % Evaluate` | `(50 / 100) / 100` | `(50 / 100) / 100` |

文字列APIの `calculate("100 + 10%")` は常に `ExpressionPercent` の意味であり、`110` にはならない。電卓型percentはsession action列からのみ生成する。

`Ans`、memory、historyは表示文字列ではなく、lowering後の厳密式、`SemanticSettings`、必要なmetadataを保存する。エラー結果では `Ans` を更新しない。`ClearEntry` は現在入力だけを消し、`Ans`、memory、historyを保持する。`ClearAll` は現在入力と未確定表示を消すが、memoryは `MemoryClear` でのみ消す。`DeleteBackward` は結果表示直後に押された場合、前回結果を文字列化して編集するのではなく、新しい空入力へ遷移する。

session transcriptの基準例は次である。

```text
actions: 1 + 2 Evaluate
command source: "1+2"
result Ans exact expression: 3

actions: Ans * 3 Evaluate
command source: "(Ans)*3"
lowered exact expression: 3 * 3
result Ans exact expression: 9

actions: 5 MemoryAdd, MemoryRecall, +, 2, Evaluate
memory exact expression: 5
command source: "(M)+2"
result exact expression: 7
```

---

## 14. サンプルUI

初期の基準実装は、依存を最小化するためVanilla TypeScriptで作る。React版は統合例とする。

サンプルUIは既定でダークテーマとする。

画面構成は次とする。

```text
┌──────────────────────────────────────┐
│ 入力式                               │
│ sin(π / 6) + sqrt(2)                 │
├──────────────────────────────────────┤
│ 厳密値                               │
│ = 1/2 + sqrt(2)                      │
│                                      │
│ 科学表記 50桁                        │
│ ≈ 1.914213562373095048801688... ×10⁰ │
│                                      │
│ 保証区間                             │
│ ∈ [lower, upper]                     │
├──────────────────────────────────────┤
│ [sin] [cos] [tan] [log] [sqrt] ...  │
└──────────────────────────────────────┘
```

機能は以下を含める。

* 厳密表示と科学表記の個別表示切替
* 有効数字数入力
* 丸めモード選択
* radian、degree切替
* 分数、帯分数、有限小数の表示選択
* 保証区間の開閉
* 使用アルゴリズム、内部精度、確定桁数の詳細表示
* キーボード操作
* 適切なARIA属性
* copy用plain text
* MathML表示
* 計算中表示
* workerの終了によるcancel

サンプルUIからprivateなWasm関数を呼ばず、一般利用者と同じnpm公開APIだけを使う。これによりサンプルが実質的な統合テストになる。

---

## 15. テスト設計

### 15.1 有理数

* 常に既約か
* 分母が正か
* 0の表現が一意か
* decimal parserが `f64` を経由していないか
* 四則演算後も不変条件が保たれるか

### 15.2 代数的数

* 隔離区間が根を一つだけ含むか
* 最小多項式がprimitiveか
* root indexと区間が一致するか
* 演算結果が入力式と同じ値か
* 次数・係数上限超過時に厳密式が失われないか

### 15.3 区間演算

すべての演算について、

```text
真値 ∈ 出力区間
```

を検査する。

MPFR、Arb、FLINTなどはproduction依存には含めず、独立したCI oracleとして利用する。FLINT自身も代数的数の次数・係数bit数に対する上限を設けたAPIを持っており、同種の制限設計は妥当である。([Flint Library][3])

### 15.4 科学表記

特に次を重点的に検査する。

```text
0
正負の値
9.999... からの桁上がり
0.999... からの指数変更
丸め境界の直前
丸め境界ちょうど
丸め境界の直後
偶数丸めの偶数側
偶数丸めの奇数側
極端に大きい指数
極端に小さい指数
```

### 15.5 特殊角

```text
sin(π/6) = 1/2
cos(π/3) = 1/2
tan(π/4) = 1
asin(1/2) = π/6    radian mode
asin(1/2) = 30     degree mode
acos(-1) = π       radian mode
acos(-1) = 180     degree mode
atan(1) = π/4      radian mode
atan(1) = 45       degree mode
tan(π/2) = TangentPole
tan(90) = TangentPole    degree mode
```

周期、象限、負角も網羅する。

### 15.6 property testとfuzzing

* parser fuzzing
* simplifier fuzzing
* expression depth攻撃
* 巨大指数入力
* 括弧不整合
* Unicode境界
* 同値変形前後の保証区間比較
* `simplify(simplify(x)) == simplify(x)` の冪等性

### 15.7 parser/session conformance

入力文法、session、percent policyはgolden fileで検査する。

```text
fixture:
    sourceまたはaction列
    parse settings
    semantic settings
    expected parse tree shape
    expected lowered exact expression
    expected error kind/span
    expected session command
    expected Ans/memory/history state
```

最低限、次を含める。

* `2^3^2`
* `-2^2`
* `2^-3`
* `2/3π`
* `2(3+4)`
* `.5` と `1.` のparse error
* `sin 30` のparse error
* `100 + 10%` の文字列API結果
* `100 + 10 % Evaluate` の `ExpressionPercent` / `CalculatorPercent` 差分
* `Ans`、memory、error後の復帰、`ClearEntry`、`ClearAll`
* UTF-8 spanとUTF-16 spanの対応

### 15.8 DTO/API conformance

DTOは生成されたTypeScript型、serde round-trip、手書きfacadeの境界検証をまとめて検査する。

* `null` / `undefined` を公開DTOとして受理しない
* `NaN` / `Infinity` / 小数 / 負数 / unsafe integerをresource limitで拒否する
* decimal string grammar違反を拒否する
* 未知の `tag` / `code` を `unsupportedProtocol` として扱う
* `Partial` が `certifiedEnclosure` を常に含む
* worker cancelが計算を中断し、破損した部分結果を成功値として返さない

### 15.9 nativeとWasmの一致

同じrequestに対するDTOをgolden fileで比較する。

```text
native serialized output
    ==
wasm serialized output
```

`wasm-bindgen-test` により `wasm32-unknown-unknown` 向けテストを通常のRustテストに近い形で実行できる。([wasm-bindgen.github.io][9])

### 15.10 sample UI integration

sample UIはpublic npm APIだけを使うことをbrowser testで確認する。

* private Wasm bindingをimportしていない
* session dispatch APIだけでbutton操作が成立する
* plain text copyが `renderPlainText` を使う
* MathML表示が `renderMathMl` を使う
* worker cancelがUI操作から到達可能である

### 15.11 静的検査

CIで少なくとも次を実行する。

```text
cargo fmt --check
cargo clippy --all-targets --all-features
cargo check -p calculator-core --no-default-features
cargo test -p calculator-core --no-default-features
cargo test
cargo test --target wasm32-unknown-unknown
cargo doc
cargo deny check
cargo xtask generate-types
cargo xtask check-generated
git diff --exit-code
pnpm --dir packages/calculator tsc --noEmit
```

さらに `calculator-core` 内で `f32`、`f64` の使用を禁止する独自検査を入れる。

---

## 16. 実装段階

### Phase 0: 契約の固定

実装対象:

* 入力文法
* 演算子優先順位
* 実数領域の定義
* 主値範囲
* Rust公開型
* DTO
* エラー型
* resource limit
* presentation tree
* protocol version
* author / license metadata
* golden test形式

完了条件:

```text
API型だけで全要件を表現できる
null / undefined に依存しない
近似値と厳密値が混同されない
部分結果を型で表現できる
```

---

### Phase 1: 有理数電卓とnpm骨格

実装対象:

* lexer
* parser
* Source AST
* expression DAG
* 任意精度整数
* 有理数
* 小数の厳密parse
* 四則演算
* 整数累乗
* exact presentation
* Wasm/npm package
* Vanilla UI

完了条件:

```text
0.1 + 0.2 = 3/10
1 / 3 + 1 / 6 = 1/2
巨大整数を損失なく処理
JavaScript numberへ値を変換しない
```

---

### Phase 2: 保証付き任意精度近似

実装対象:

* float backend wrapper
* exact dyadic変換
* interval arithmetic
* π、eの保証区間
* exp、log、sin、cos、tan
* adaptive precision
* 科学表記
* worker API

完了条件:

```text
指定桁が一意に確定するまで自動精度増加
すべての小数表示に保証区間が存在
精度不足時に推測値を返さない
```

---

### Phase 3: 簡約と特殊値

実装対象:

* proof predicates
* guarded rewrite
* 根号
* (\pi) の有理数倍
* 特殊角
* 逆三角関数の既知値
* exact display candidate selection

完了条件:

```text
sin(π/6) = 1/2
asin(1/2) = π/6    radian mode
asin(1/2) = 30     degree mode
sqrt(72) = 6sqrt(2)
sqrt(x²) を無条件に x へしない
数値的近さから厳密値を推測しない
```

---

### Phase 4: 一般実代数的数

実装対象:

* integer polynomial
* Sturm sequence
* root isolation
* resultant
* polynomial factorization
* minimal polynomial
* algebraic comparisons
* cyclotomic exact trig

完了条件:

```text
代数的数の構造的等値判定
演算後の最小多項式正規化
根の一意性を隔離区間で保証
上限超過時に形式式へ安全に戻る
```

---

### Phase 5: 1.0向け堅牢化

実装対象:

* native/Wasm differential tests
* 外部oracle CI
* fuzzing
* browser integration tests
* package size最適化
* API surface検査
* protocol snapshot regression
* ドキュメントテスト
* セキュリティ監査

完了条件:

```text
同一入力・設定から同一DTOを生成
warm cacheとcold cacheで結果が同一
全公開enumの分岐が網羅的
公開契約と現在の実装詳細が文書上分離
```

---

## 採用する全体設計

中心となる設計は次である。

```text
Exact Expression DAG
    ├── Rational recognition
    ├── Real Algebraic recognition
    ├── Rational × π recognition
    └── General Symbolic retention

Exact Expression DAG
    ↓
Certified Interval Evaluator
    ↓
Adaptive Decimal Rounding
    ↓
Exact / Scientific / Enclosure presentations
```

coreは `#![no_std] + alloc` の純Rustとし、Wasm、npm、UIは外層へ置く。厳密式を常に正本として保持し、保証区間を直交する情報として追加する。一般代数的数の計算が上限を超えた場合も、近似値へ破壊的に変換せず、厳密形式式と保証区間を返す。この構造が提示された全要件を最も一貫して満たす。

[1]: https://zenn.dev/bem130/articles/1b352797de94e7 "https://zenn.dev/bem130/articles/1b352797de94e7"
[2]: https://docs.rs/num-bigint/latest/num_bigint/ "https://docs.rs/num-bigint/latest/num_bigint/"
[3]: https://flintlib.org/doc/qqbar.html "https://flintlib.org/doc/qqbar.html"
[4]: https://docs.rs/astro-float/latest/astro_float/ "https://docs.rs/astro-float/latest/astro_float/"
[5]: https://docs.rs/astro-float/latest/astro_float/enum.RoundingMode.html "https://docs.rs/astro-float/latest/astro_float/enum.RoundingMode.html"
[6]: https://docs.rs/gmp-mpfr/latest/gmp_mpfr/ "https://docs.rs/gmp-mpfr/latest/gmp_mpfr/"
[7]: https://wasm-bindgen.github.io/wasm-bindgen/reference/deployment.html "Deployment - The `wasm-bindgen` Guide"
[8]: https://docs.rs/serde-wasm-bindgen/latest/serde_wasm_bindgen/ "serde_wasm_bindgen - Rust"
[9]: https://wasm-bindgen.github.io/wasm-bindgen/wasm-bindgen-test/index.html "Testing with wasm-bindgen-test - The `wasm-bindgen` Guide"
