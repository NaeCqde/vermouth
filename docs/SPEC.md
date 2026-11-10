# vermouth 仕様

VeraCrypt で暗号化したコンテナのパスワードを、自宅のサーバーから配り、パソコンの起動後に自動でマウントするための仕組み。
3 つの Rust プログラムからなる。仕様を変えたときは、このファイルも必ず更新する。

| ディレクトリ | 役割 |
|---|---|
| `server/` | パスワードを配る HTTP サーバー(`vermouth-server`、ntex) |
| `client/` | パスワードを取得して VeraCrypt を動かすクライアント(`linux.sh` / `windows.bat` を呼ぶ) |
| `nanokvm/` | NanoKVM(RISC-V の KVM 機器)の上で動き、パソコンの電源オンを検知して、パスワードを USB キーボードとして入力する |

`nanokvm/directory/` は NanoKVM に置くファイルの一式(`/data/vermouth/` の実行ファイルと `.env`、`/etc/init.d/S99vermouth.sh`、`/etc/kvm/scripts/input-password.sh`)。実行ファイル `vermouth-nanokvm` は RISC-V 向けにビルドしたものをリポジトリに入れている。

## サーバー(`vermouth-server HOST PORT`)

起動: `vermouth-server <ホスト> <ポート>`。引数が違えば使い方を出して終了。ワーカーは 1。

設定 `config.json`(作業ディレクトリ。なければ起動時にエラーで終了):

```json
{ "token": "…", "containers": { "<クライアントのIP>": { "source": "…", "mount": "…", "password": "…" } } }
```

- `token`: クライアントが `Authorization` ヘッダーに入れる合言葉。
- `containers`: クライアントの IP アドレスごとの、コンテナのパス(`source`)、マウント先(`mount`)、パスワード(`password`)。

エンドポイント: **`POST /` だけ**。
- `Authorization` ヘッダーの値が `token` と一致しなければ、そのルートに当たらない(404)。
- 一致したら、接続元の IP(`peer_addr`)を `containers` から引く。あれば `{source, mount, password}` を JSON で返す(200)。なければ 404 で本文 `{}`。
- 接続元の IP を標準出力に出す。

## クライアント(`client/`)

`.env`(または環境変数): `SERVER`(サーバーの URL)、`TOKEN`。
1. `POST $SERVER` に `Authorization: <TOKEN>` を付けて、`{source, mount, password}` を受け取る。200 以外はエラーで終了(終了コード 1)。
2. 環境変数 `SOURCE`、`MOUNT`、`PASSWORD` を渡して、スクリプトを実行し、終わりを待つ。
   - Linux/macOS: `sh run.sh`(`linux.sh` の中身は `echo $PASSWORD | veracrypt -t $SOURCE $MOUNT`)
   - Windows: `cmd /c run.bat`(`windows.bat` は `VeraCrypt.exe /q /v "%SOURCE%" /l %MOUNT% /p "%PASSWORD%"`)

## NanoKVM(`nanokvm/`)

`.env`: `SERVER`、`TOKEN`(サーバーの取得用。クライアントと同じ)、`WAIT_TIME`(秒、必須)、`NANOKVM_SERVER`、`NANOKVM_USERNAME`、`NANOKVM_PASSWORD`、`MANUAL`(空でなければ手動)。

- **手動(`MANUAL` あり)**: パスワードを取得して入力し、終了する(`input-password.sh` が `MANUAL=true` で実行する)。
- **自動(既定。`S99vermouth.sh` が起動時に実行)**: 1 秒ごとに `/sys/class/gpio/gpio504/value` を読み、値が `0` で始まればパソコンの電源オンとみなす。電源が入った瞬間(前回は切、今回は入)に、
  1. NanoKVM の `POST /api/hid/reset` で HID をリセットし、
  2. `WAIT_TIME` 秒待ち、
  3. サーバーからパスワードを取得して、`POST /api/hid/paste`(`content` = パスワード + 改行、`langue` = `en`)で入力する。
  
  失敗は終了コード 1 で終了する。電源が切れたときはログに出すだけ。

## セキュリティ上の注意(現状)

- **サーバーとの通信は平文の HTTP。** パスワードが、経路で盗み見られうる。信頼できる LAN の中だけで使うこと。可能なら TLS にする。
- クライアントの識別は接続元 IP とトークンだけ。同じ LAN で IP を偽られると、他のクライアントのパスワードを引かれうる。
- トークンは固定の合言葉で、比較は通常の文字列比較。
- パスワードは環境変数でスクリプトに渡る。Windows 版は VeraCrypt の引数(`/p`)に載るため、プロセス一覧から見える。Linux 版も `echo $PASSWORD` で扱う(変数の引用がない)。
- `config.json` と `.env` には秘密が入る。リポジトリに入れない(`.gitignore` で除外する)。
- NanoKVM の `NANOKVM_USERNAME` / `NANOKVM_PASSWORD` は `.env` にあるが、コードでは使っていない(`/api/hid/*` へ認証なしで送っている)。
