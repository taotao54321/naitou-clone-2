# FC『内藤九段将棋秘伝』思考エンジンのシミュレーター

原作の思考エンジンを再現し、最短手数探索や思考ログ出力を行う。

## 動作環境

AVX2 命令に対応した x64 環境でのみ動作する。

## 実行

SFEN 棋譜形式については [将棋所のドキュメント](http://shogidokoro.starfree.jp/usi.html) などを参照。

原作では、平手の場合時間制限設定により戦型が変わる。これは `--timelimit` オプションで設定できる。

`emu` feature は Linux でのみ動作確認している。

### シミュレーターの思考ログを出力

`trace` バイナリを使う。引数に SFEN 棋譜を与えると、標準出力に思考ログが出力される。

```sh
$ cargo run --bin trace -- 'startpos moves 7g7f 3c3d 8h2b+ 3a2b 1i1h B*1i 5i6h 8b7b 3i4h 1i2h+ 4g4f 2h2i B*1e' > log.txt
```

### 最短勝利手順を求める

`solve` バイナリを使う。引数に SFEN 棋譜と探索深さを与えると、標準出力に全ての解が出力される。  
深さ 1 がプレイヤー側の 1 手に相当する。たとえば、先手の 13 手以下の最短勝利手順を求めたければ深さに 7 を指定する。

```sh
$ cargo run --profile release-lto --bin solve -- 'startpos moves 7g7f 3c3d 8h2b+ 3a2b 1i1h B*1i 5i6h 8b7b' 3 > solution.txt
```

デフォルトのスレッド数は論理 CPU 数となる。これは `--thread-count` オプションで変更できる。

スレッドごとの探索ノード数をなるべく均等にするため、一定の深さまでは全スレッドで探索を行う。  
デフォルトでは探索深さの半分(切り上げ)に達したらスレッドごとの分岐を開始する。これは `--branch-depth` オプションで変更できる。

中程度のマシンスペックがあれば、初形からの深さ 7 (先手なら13手、後手なら14手)の探索は 1 日程度で終わるはず。

### 最短全駒勝利手順を求める

`solve_extinct` バイナリを使う。引数に SFEN 棋譜と探索深さを与えると、標準出力に全ての解が出力される。  
全駒の場合、COM の残り駒数による枝刈りが効くため、ある程度深さが大きくても高速に解けることがある。

```sh
$ cargo run --profile release-lto --bin solve_extinct -- --timelimit 'startpos moves 7g7f 3c3d 8h7g 2b7g+ 8i7g 4c4d B*1f B*8i 1f3d 3a3b 2h8h 8b7b 8h8i 5a6b B*1f 5c5d 3d6a 7b8b 6a2e 2c2d 2e6a+ 6b5c 6a7a 8b6b 1f3d 3b4c 3d4c 2a3c 4c5d+ 5c5b 5d4d 6b6a 4d3d 5b4b 7a6a 4a5a R*8b 4b4a' 11 > solution.txt
```

`solve` バイナリと同様のオプションを受け付ける。

局面にもよるが、中程度のマシンスペックがあれば、駒取りでない手が 4 手前後までなら 1 日以内に解けるはず。

### 原作の思考ログを出力 (要 `emu` feature)

`emu_trace` バイナリを使う。引数に原作の ROM ファイルと SFEN 棋譜を与えると、エミュレーター上で棋譜が再生され、標準出力に思考ログが出力される。

```sh
$ cargo run --features emu,sdl --bin emu_trace -- path/to/naitou.nes 'startpos moves 7g7f 3c3d 8h2b+ 3a2b 1i1h B*1i 5i6h 8b7b 3i4h 1i2h+ 4g4f 2h2i B*1e' > log.txt
```

### シミュレーターと原作の思考ログが一致するかテストする (要 `emu` feature)

`verify` バイナリを使う。引数に原作の ROM ファイルと SFEN ファイル群を与えると、全棋譜について思考ログが一致するかテストする。  
一致しなかった思考ログたちは `log/` ディレクトリに出力される。

```sh
$ cargo run --bin verify -- path/to/naitou.nes tests/asset/verify/notimelimit/HUM先手-15手勝利.sfen
```

### 棋譜を FCEUX 用ムービー (`.fm2`) に変換する (要 `emu` feature)

`kifu_to_movie` バイナリを使う。引数に原作の ROM ファイルと SFEN 棋譜を与えると、それを最速で入力する `.fm2` ムービーが標準出力に出力される。

```sh
$ cargo run --features emu --bin kifu_to_movie -- path/to/naitou.nes 'startpos moves 7g7f 3c3d 8h2b+ 3a2b 1i1h B*1i 5i6h 8b7b 3i4h 1i2h+ 4g4f 2h2i B*1e' > movie.fm2
```

## Credits

bitboard, 利きの差分更新などのコードは [やねうら王](https://github.com/yaneurao/YaneuraOu) を参考にしている。

## License

GPLv3
