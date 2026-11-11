# vermouth 仕様

VeraCrypt で暗号化したコンテナのパスワードを、自宅のサーバーから配り、パソコンの起動後に自動でマウントするための仕組み。
3 つの Rust プログラムからなる。仕様を変えたときは、このファイルも必ず更新する。

| ディレクトリ | 役割 |
|---|---|
| `server/` | パスワードを配る HTTP サーバー(`vermouth-server`、ntex) |
| `shared/` | 3 つに共通のクレート `vermouth_common`。答えの暗号化と署名(`Keys`)、サーバーからの取得(`fetch_container`、feature `client`) |
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
{ "token": "…", "listen": "0.0.0.0:9810", "containers": { "<クライアントのIP>": { "source": "…", "mount": "…", "password": "…" } } }
```

- `token`: クライアントとの共有の秘密。通信の暗号化と署名の鍵のもと(下の「通信の暗号化」)。**長いランダムな値にする**(`openssl rand -base64 32`)。空なら起動時にエラーで終了する。24 文字より短いと起動時に警告を出す。
- `listen`: 引数なしで起動したときに待ち受ける `host:port`(省略可。環境変数 `LISTEN` が優先)。
- `containers`: クライアントの IP アドレスごとの、コンテナのパス(`source`)、マウント先(`mount`)、パスワード(`password`)。

エンドポイント: **`POST /` だけ**。
- ヘッダー `x-vermouth-nonce` と `Authorization`(署名)が無い、または署名が合わなければ 404(本文なし)。
- 合ったら、接続元の IP(`peer_addr`)を `containers` から引く。あれば `{source, mount, password}` の JSON を**暗号化して**返す(200、`application/octet-stream`)。なければ 404。

### 通信の暗号化

HTTP のままでも、経路で見られてパスワードが分からないようにしてある(`shared/`)。`token` から HKDF-SHA256 で署名用と暗号化用の 2 つの鍵を作る。

- リクエスト: ランダムな nonce(16 バイト、base64)をヘッダー `x-vermouth-nonce` に、`HMAC-SHA256(署名鍵, "vermouth v1 request" ‖ nonce)`(base64)を `Authorization` に入れる。`token` そのものは送らない。
- 答え: JSON を XChaCha20-Poly1305(暗号化鍵)で暗号化し、先頭に乱数の 24 バイトの nonce を付ける。リクエストの nonce を関連データにするので、別のリクエストへの答えを流し込んでも開けない(再生攻撃の防止)。
- 守れるもの: 盗み見(パスワードは見えない)、答えの改ざん、答えの差し替え。守れないもの: どのクライアントがいつ問い合わせたかという通信の存在、サーバーの停止などの妨害。`token` が漏れれば破られるので、長いランダムな値にすること。
- TLS ではないので、証明書は要らない。
- 接続元の IP を標準出力に出す。

## クライアント(`client/`)

`.env`(または環境変数): `SERVER`(サーバーの URL)、`TOKEN`。
1. `POST $SERVER` に署名付きのリクエストを送り、暗号化された答えを開いて `{source, mount, password}` を得る。200 以外、または開けなければエラーで終了(終了コード 1)。
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

- サーバーとの通信は HTTP のままだが、答え(パスワード)は `token` から作った鍵で暗号化・認証している(上の「通信の暗号化」)。経路で見られても、`token` を知らなければパスワードは分からない。通信の存在そのものは見える。
- **このバージョンで通信の形式が変わった。** 古いクライアント・古い `vermouth-nanokvm` は新しいサーバーと話せない(サーバーは 404)。全部を同じバージョンにすること。`nanokvm/directory/data/vermouth/vermouth-nanokvm` は古い形式のままなので、リリースの `vermouth-nanokvm-Linux-riscv64` で置き換える。
- クライアントの識別は接続元 IP とトークンだけ。同じ LAN で IP を偽られると、他のクライアントのパスワードを引かれうる。
- トークンは固定の共有の秘密。署名の比較は定数時間。
- パスワードは環境変数でスクリプトに渡る。Windows 版は VeraCrypt の引数(`/p`)に載るため、プロセス一覧から見える。Linux 版も `echo $PASSWORD` で扱う(変数の引用がない)。
- `config.json` と `.env` には秘密が入る。リポジトリに入れない(`.gitignore` で除外する)。
- NanoKVM の `NANOKVM_USERNAME` / `NANOKVM_PASSWORD` は `.env` にあるが、コードでは使っていない(`/api/hid/*` へ認証なしで送っている)。
