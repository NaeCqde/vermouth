# Vermouth

日本語 | [English](README.en.md)

VeraCrypt で暗号化したコンテナのパスワードを、自宅のサーバーから配り、パソコンの起動後に自動でマウントするための仕組みです。3 つの Rust プログラムからなります。動作の決まりは [docs/SPEC.md](docs/SPEC.md) にあります。

## サーバー

パスワードを配る HTTP サーバー(`vermouth-server`)です。`vermouth-server <ホスト> <ポート>` で起動します。設定の雛形は `server.example.config.json` です。待ち受けるアドレスは、引数 `<ホスト> <ポート>`、環境変数 `LISTEN=host:port`、`config.json` の `"listen"` のどれでも決められます。

通信は、答え(パスワード)を `token` から作った鍵で暗号化・認証してあります(TLS ではなく、証明書は不要です)。`token` は `openssl rand -base64 32` のような長いランダムな値にしてください。クライアントとサーバーのバージョンは揃えてください(古いクライアントは新しいサーバーと話せません)。

## クライアント

パスワードを取得して VeraCrypt を動かすクライアント(`vermouth`)です。Linux は `client.linux.sh`、Windows は `client.windows.bat` を呼びます。設定の雛形は `client.example.env` です。

## NanoKVM

NanoKVM(RISC-V の KVM 機器)の上で動き、パソコンの電源オンを検知して、パスワードを USB キーボードとして入力します(`vermouth-nanokvm`)。設定の雛形は `nanokvm.example.env` です。

### セットアップ

NanoKVM に置くファイルの一式は `nanokvm/directory/` にあります(`/data/vermouth/` の実行ファイルと `.env`、`/etc/init.d/S99vermouth.sh`、`/etc/kvm/scripts/input-password.sh`)。詳しくは [docs/SPEC.md](docs/SPEC.md) を見てください。

## ダウンロード

リリースに、各プラットフォーム(Linux x86_64 / aarch64 / riscv64、Windows x86_64、macOS aarch64)の実行ファイルが置かれます。
