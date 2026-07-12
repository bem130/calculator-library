# Issue 17: canonical Rational reciprocalをinterval/atanへ一般化する

Interval除算と`atan(x>1)`はcanonical非零Rationalの逆数を汎用除算で再正規化する。
符号を正規化して分子・分母を交換すれば互いに素性を保てるため、重複GCDを除く。

- 正負canonical値、zero拒否、interval端点順序、atan identityを回帰する。
- atan benchmark/allocation caseを追加しbefore/afterを記録する。
- 公開契約、directed bounds、logical-workを維持し、全gate/review後に一度だけ統合する。
