# Issue 44: asinの`1-x^2`をcanonical partsから直接構築する

asin変換域は各endpointで`x*x`をcanonical化し、その結果をRational 1から減算して
再度canonical化する。canonical `x=n/d`では`1-x^2=(d^2-n^2)/d^2`なので、
BigInt partsから一度だけRationalを構築できる。

- zero、half境界、1近傍、exact 1、正負値で旧乗算・減算とexact一致させる。
- sqrt domain、directed asin/acos bounds、logical work、resource、protocolを維持する。
- transformed asin/acos allocationをbefore/after測定し、全gate/review後に統合する。
