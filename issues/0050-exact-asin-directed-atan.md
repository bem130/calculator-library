# Issue 50: exact transformed asinのatanを方向付き評価する

exact transformed asinはsqrt enclosureの上下比ごとにpaired atan boundsを構築し、
lower側とupper側を一つずつ捨てる。`pi/2-atan`の減数方向に必要な片側だけを
directed evaluatorで構築し、exact asin/acosの保証区間を維持する。

- 正負、half/unit境界で旧paired定義とexact一致させる。
- logical work、resource、protocolを維持する。
- `acos(3/4)`のallocation/timingと公開境界を測定し、全gate/review後に統合する。
