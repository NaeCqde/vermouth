# vermouth 仕様

VeraCrypt で暗号化したコンテナのパスワードを、自宅のサーバーから配り、パソコンの起動後に自動でマウントするための仕組み。
3 つの Rust プログラムからなる。仕様を変えたときは、このファイルも必ず更新する。

| ディレクトリ | 役割 |
|---|---|
| `server/` | パスワードを配る HTTP サーバー(`vermouth-server`、ntex) |
| `shared/` | 3 つに共通のクレート `vermouth_common`。ハンドシェイクと暗号化(`ClientHandshake`、`ServerAnswer`)、サーバーからの取得(`fetch_container`、feature `client`) |
| `client/` | パスワードを取得して VeraCrypt を動かすクライアント(`linux.sh` / `windows.bat` を呼ぶ) |
| `nanokvm/` | NanoKVM(RISC-V の KVM 機器)の上で動き、パソコンの電源オンを検知して、パスワードを USB キーボードとして入力する |

`nanokvm/directory/` は NanoKVM に置くファイルの一式(`/data/vermouth/` の実行ファイルと `.env`、`/etc/init.d/S99vermouth.sh`、`/etc/kvm/scripts/input-password.sh`)。実行ファイル `vermouth-nanokvm` は RISC-V 向けにビルドしたものをリポジトリに入れている。

## サーバー(`vermouth-server [HOST PORT]`)

待ち受けるアドレスは、次の順で最初に見つかったものになる。
1. 引数 `<ホスト> <ポート>`(これまでどおり)
2. 環境変数 `LISTEN`(`host:port`。IPv6 は `[::]:9810`)
3. `config.json` の `"listen"`(`host:port`)

どれもなければエラーで終了する。引数の数が 0 でも 2 でもなければ使い方を出して終了する。ワーカーは 1。

設定 `config.json`(作業ディレクトリ。なければ起動時にエラーで終了):

```json
{ "server_key": "…", "listen": "0.0.0.0:9810", "devices": { "<端末の名前>": { "key": "…", "source": "…", "mount": "…", "password": "…" } } }
```

- `server_key`: サーバーの秘密鍵(base64)。`vermouth-server server-key` で作る。config.json は他人に読ませない。
- `listen`: 引数なしで起動したときに待ち受ける `host:port`(省略可。環境変数 `LISTEN` が優先)。
- `devices`: 端末ごとの設定。`key` はその端末の**公開鍵**(base64。token から作った一方向の値で、token そのものは保存しない。`vermouth-server token` または `vermouth-server key` で作る)。`source`(コンテナのパス)、`mount`(マウント先)、`password`(パスワード)。

補助コマンド(config.json は不要): `vermouth-server token`(端末用の新しい TOKEN と、config.json 用の `key` を出す)、`vermouth-server key`(標準入力の token から `key` を出す)、`vermouth-server server-key`(サーバーの鍵対を出す)。

エンドポイント: **`POST /` だけ**。本文はハンドシェイクの第 1 メッセージ(バイナリ、256 バイトまで)。
- 開けない(このサーバーの鍵あてでない、壊れている、長すぎる)、または `devices` に鍵がなければ 404(本文なし)。
- 開けて、端末が見つかったら、`{source, mount, password}` の JSON を載せた第 2 メッセージを返す(200、`application/octet-stream`)。接続元の IP は端末の判定に**使わない**。

### 通信の暗号化

HTTP の上で Noise の `IK` ハンドシェイク(`Noise_IK_25519_ChaChaPoly_BLAKE2s`、`shared/`)を 1 往復で行う。

- 端末ごとに固有の `TOKEN`。端末の鍵対は token から HKDF-SHA256 で作る。サーバーは公開鍵(`key`)だけを持つので、config.json が漏れても token は分からず、端末になりすませない。token は長いランダムな値にする(`vermouth-server token`。32 文字より短いと推測される)。
- クライアントはサーバーの公開鍵(`SERVER_KEY`)を固定(ピン留め)して持つ。秘密鍵を持たない者は答えられないので、経路上の偽サーバーは拒否される。
- 双方が静的鍵で互いを認証し、通信の鍵は双方の一時鍵から作る(前方秘匿性: 後で token やサーバー鍵が漏れても、記録した通信は開けない)。端末の公開鍵は暗号化されて送られ、経路からは見えない。
- 記録したリクエストを経路で再送されても、答えを開けるのはそのリクエストを作った本人だけ。改ざん・差し替えは認証で弾かれる。
- 守れるもの: 盗み見、改ざん、なりすまし(サーバー側も端末側も)、再送、接続元 IP の偽装。守れないもの: 通信の存在とそのタイミング・大きさ、通信を落とすなどの妨害。端末の token とサーバーの秘密鍵は守ること。
- TLS ではないので、証明書は要らない。
- 端末の名前をログに出す。

## クライアント(`client/`)

`.env`(または環境変数): `SERVER`(サーバーの URL)、`TOKEN`(この端末の token)、`SERVER_KEY`(サーバーの公開鍵)。
1. `POST $SERVER` にハンドシェイクの第 1 メッセージを送り、第 2 メッセージを開いて `{source, mount, password}` を得る。200 以外、または開けなければエラーで終了(終了コード 1)。
2. 環境変数 `SOURCE`、`MOUNT`、`PASSWORD` を渡して、スクリプトを実行し、終わりを待つ。
   - Linux/macOS: `sh run.sh`(`linux.sh` の中身は `echo $PASSWORD | veracrypt -t $SOURCE $MOUNT`)
   - Windows: `cmd /c run.bat`(`windows.bat` は `VeraCrypt.exe /q /v "%SOURCE%" /l %MOUNT% /p "%PASSWORD%"`)

## NanoKVM(`nanokvm/`)

`.env`: `SERVER`、`TOKEN`、`SERVER_KEY`(サーバーの取得用。クライアントと同じ)、`WAIT_TIME`(秒、必須)、`NANOKVM_SERVER`、`NANOKVM_USERNAME`、`NANOKVM_PASSWORD`、`MANUAL`(空でなければ手動)。

- **手動(`MANUAL` あり)**: パスワードを取得して入力し、終了する(`input-password.sh` が `MANUAL=true` で実行する)。
- **自動(既定。`S99vermouth.sh` が起動時に実行)**: 1 秒ごとに `/sys/class/gpio/gpio504/value` を読み、値が `0` で始まればパソコンの電源オンとみなす。電源が入った瞬間(前回は切、今回は入)に、
  1. NanoKVM の `POST /api/hid/reset` で HID をリセットし、
  2. `WAIT_TIME` 秒待ち、
  3. サーバーからパスワードを取得して、`POST /api/hid/paste`(`content` = パスワード + 改行、`langue` = `en`)で入力する。
  
  失敗は終了コード 1 で終了する。電源が切れたときはログに出すだけ。

## セキュリティ上の注意(現状)

- サーバーとの通信は HTTP の上で Noise IK のハンドシェイクを行い、端末ごとの token とサーバーの公開鍵のピン留めで相互に認証し、暗号化している(上の「通信の暗号化」)。経路(ルーター)が乗っ取られても、パスワードは読めず、サーバーにもなりすませない。
- **このバージョンで通信の形式が変わった。** 古いクライアント・古い `vermouth-nanokvm` は新しいサーバーと話せない(サーバーは 404)。全部を同じバージョンにすること。`nanokvm/directory/data/vermouth/vermouth-nanokvm` は古い形式のままなので、リリースの `vermouth-nanokvm-Linux-riscv64` で置き換える。
- クライアントの識別は接続元 IP とトークンだけ。同じ LAN で IP を偽られると、他のクライアントのパスワードを引かれうる。
- トークンは固定の共有の秘密。署名の比較は定数時間。
- パスワードは環境変数でスクリプトに渡る。Windows 版は VeraCrypt の引数(`/p`)に載るため、プロセス一覧から見える。Linux 版も `echo $PASSWORD` で扱う(変数の引用がない)。
- `config.json` と `.env` には秘密が入る。リポジトリに入れない(`.gitignore` で除外する)。
- NanoKVM の `NANOKVM_USERNAME` / `NANOKVM_PASSWORD` は `.env` にあるが、コードでは使っていない(`/api/hid/*` へ認証なしで送っている)。
