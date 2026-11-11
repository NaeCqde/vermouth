# Vermouth

日本語 | [English](README.en.md)

VeraCrypt で暗号化したコンテナのパスワードを、自宅のサーバーから配り、パソコンの起動後に自動でマウントするための仕組みです。3 つの Rust プログラムからなります。動作の決まりは [docs/SPEC.md](docs/SPEC.md) にあります。

## サーバー

パスワードを配る HTTP サーバー(`vermouth-server`)です。`vermouth-server <ホスト> <ポート>` で起動します。設定の雛形は `server.example.config.json` です。待ち受けるアドレスは、引数 `<ホスト> <ポート>`、環境変数 `LISTEN=host:port`、`config.json` の `"listen"` のどれでも決められます。

通信は、HTTP の上で Noise `IK` のハンドシェイクを行い、端末ごとの `TOKEN` とサーバーの公開鍵のピン留め(`SERVER_KEY`)で相互に認証して暗号化します(TLS ではなく、証明書は不要です)。サーバーの `config.json` には token ではなくその公開鍵(`key`)だけを保存します。ルーターが乗っ取られても、盗み見・改ざん・なりすまし・再送はできません。`vermouth-server token` で端末用の token と `key`、`vermouth-server server-key` でサーバーの鍵を作れます。クライアントとサーバーのバージョンは揃えてください(古いクライアントは新しいサーバーと話せません)。

## クライアント

パスワードを取得して VeraCrypt を動かすクライアント(`vermouth`)です。Linux は `client.linux.sh`、Windows は `client.windows.bat` を呼びます。設定の雛形は `client.example.env` です。

## NanoKVM

NanoKVM(RISC-V の KVM 機器)の上で動き、パソコンの電源オンを検知して、パスワードを USB キーボードとして入力します(`vermouth-nanokvm`)。設定の雛形は `nanokvm.example.env` です。

### セットアップ

NanoKVM に置くファイルの一式は `nanokvm/directory/` にあります(`/data/vermouth/` の実行ファイルと `.env`、`/etc/init.d/S99vermouth.sh`、`/etc/kvm/scripts/input-password.sh`)。詳しくは [docs/SPEC.md](docs/SPEC.md) を見てください。

## ダウンロード

リリースに、各プラットフォーム(Linux x86_64 / aarch64 / riscv64、Windows x86_64、macOS aarch64)の実行ファイルが置かれます。
