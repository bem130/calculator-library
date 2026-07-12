# Issue 22: unit-range sin/cosで不要な対成分評価を除く

`sin_rational` / `cos_rational` は一方の成分だけを返すが、`|x| <= 1`でも
`sin_cos_rational`を通じてsin/cos両級数と角加算ペアを構築し、片方を捨てている。

- unit-rangeの単独sin/cosは対応する方向付き級数boundから直接intervalを構築する。
- `|x| > 1`のrange reductionと、両成分が必要なtanのpaired pathは維持する。
- 正負、境界1、direct pathとpaired pathのexact interval一致を回帰する。
- `sin(1)` / `cos(1)`のnative・Wasm改善と`tan(1)`の非退行を測定する。
- logical work、resource limit、no-float、公開protocolを変えず、全gate/review後に一度だけ統合する。
