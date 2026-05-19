---
title: "Runtime Path と App Data Path"
description: "GDLauncher が使う 2 種類のパス。それぞれが何を保持しているか、なぜ分かれているか、そしてどちらを移動するべきか。"
faq:
  - question: "App Data Path と Runtime Path の違いは？"
    answer: "App Data Path は Electron がユーザーごとに標準で使うディレクトリで、キャッシュや runtime_path_override マーカーが入ります。Runtime Path は GDLauncher のコアが重いもの (インスタンス、Minecraft アセットとライブラリ、Java インストール、ランチャーのデータベース、アプリ全体のログ) を置く場所。デフォルトでは Runtime Path は App Data Path の中にありますが、移動できるのは Runtime Path の方です。"
  - question: "App Data Path はどこ？"
    answer: "Windows: C:\\Users\\<ユーザー名>\\AppData\\Roaming\\gdlauncher_carbon。macOS: /Users/<ユーザー名>/Library/Application Support/gdlauncher_carbon。Linux: $XDG_DATA_HOME/gdlauncher_carbon、未設定なら ~/.local/share/gdlauncher_carbon。"
  - question: "どちらを移動するべき？"
    answer: "Runtime Path です。インスタンスを追加するたびに大きくなるのはこちら。App Data Path は小さいまま OS のユーザープロファイルに紐付きます。GDLauncher は Settings → Runtime Path で Runtime Path の移動を提供しますが、App Data Path は Electron 管理で UI から動かせません。"
  - question: "runtime_path_override ファイルは何？"
    answer: "Runtime Path を変更すると、GDLauncher は App Data Path 内に runtime_path_override という小さなテキストファイルを書き込みます。中身は新しい Runtime Path の文字列で、起動のたびにこれを読んでデータの場所を判断します。削除するとデフォルトの Runtime Path にフォールバックします。"
  - question: "Runtime Path を 2 台の PC で共有できる？"
    answer: "できません。データベースには PC ごとの状態 (パス、トークン、Java インストール) が保存されており、同時利用は想定されていません。別の PC で同じインスタンスが欲しい場合は手動でコピーするか、Cloud Instance Share 機能を使ってください。"
---

# Runtime Path と App Data Path

## 2 つのパス、2 つの役割

GDLauncher はファイルを 2 か所に分けて置きます。Electron 側の小さなもの用の **App Data Path**、本体のデータ用 (インスタンス、アセット、Java、データベース) の **Runtime Path**。たまに移動したくなるのは Runtime Path で、App Data Path はほぼ触る機会がありません。

### App Data Path

OS 標準のユーザーアプリディレクトリ、つまり Electron の `userData` が指す場所です。GDLauncher は次の用途に使います:

- `runtime_path_override` マーカー (Runtime Path の実際の場所を伝える 1 行のテキストファイル)
- 未移動の場合は `data/` サブフォルダーがデフォルトの Runtime Path
- Electron 自身の Chromium キャッシュ (Network/、GPUCache/、Cookies など)
- Electron メインプロセスのログ

OS 標準の場所に置かれます:

- **Windows:** `C:\Users\<ユーザー名>\AppData\Roaming\gdlauncher_carbon`
- **macOS:** `/Users/<ユーザー名>/Library/Application Support/gdlauncher_carbon`
- **Linux:** `$XDG_DATA_HOME/gdlauncher_carbon`、未設定なら `~/.local/share/gdlauncher_carbon`

`data/` サブフォルダーを除けば通常は小さなディレクトリです。GDLauncher にはこれを移動する設定はなく、OS の規約と Electron が現状の場所を前提にしています。

### Runtime Path (Core Module)

GDLauncher の Rust コアが、それ以外のすべてを置く場所:

- インスタンス (`instances/` 配下)
- Minecraft 共有アセット (Mojang 配布のテクスチャ、サウンド)
- Minecraft 共有ライブラリ (Mojang と Mod ローダーの JAR)
- GDLauncher がダウンロードした Java
- ランチャーのデータベース `gdl_conf.db`
- アプリ全体のログ (`__gdl_logs__/`)

デフォルトでは `<App Data Path>/data/` に置かれるので、App Data の中にあります。**Settings → Runtime Path** で任意の場所に変更できます。大型 Modpack を入れるとあっという間に 50 GB を超えるパスです。

## Runtime Path を移動したいケース

主な理由は 2 つ:

1. **SSD が満杯。** インスタンスを大容量 HDD やセカンダリ SSD に移したい。
2. **OS ユーザープロファイルとは別に管理したい。** 独立してバックアップ管理したいときに有効。ただしプレイ中の同期はファイルロックでぶつかるので避けてください。

通常用途では Runtime Path を動かす必要はありません。デフォルトの場所で問題ありません。

## 移動の仕組み

**Settings → Runtime Path** を開きます。新しい場所を入力するか、フォルダーアイコンから選択。現在のパスと違い、かつ有効な場所であれば適用ボタン (行の右側の円形矢印アイコン) が点灯します。クリックすると、旧パスと新パスを表示する確認モーダルが開きます。

移動先のフォルダーが空 (または未作成) なら、確認すると本格的な移行が走ります。オーバーレイがスキャン、ファイルごとのコピー、ソースのファイルごとの削除を順に表示します。実行中にアプリやマシンを終了しないでください。完了後にランチャーが自動で再起動します。

移動先に既に GDLauncher の Runtime Path がある場合 (一度移動済みで、新しいインストールから同じデータを使いたい等)、モーダルが黄色で「空ではない」と警告します。承認するとマーカーが既存のデータを指すよう書き換えられるだけで、ファイルコピーは走らずランチャーが再起動します。旧位置のデータは孤立した状態で残るので、手動で削除して構いません。

途中で移行が失敗した場合、オーバーレイが赤くなりエラーを表示します。ランチャーがロールバックし、新パスに作成したファイルは削除され、マーカーは旧パスを指したままなので、データを失わずにやり直せます。よくある原因は、移動先ドライブの書き込み権限不足と、空き容量不足です。

### runtime_path_override マーカー

Runtime Path を変更すると、GDLauncher は **App Data Path** の中 (Runtime Path の中ではなく) に `runtime_path_override` というテキストファイルを作ります。中身は新しい Runtime Path のプレーンテキスト。起動のたびにこのファイルを読んでデータの場所を判断します。

誤って削除すると、GDLauncher はデフォルトの Runtime Path (`<App Data>/data/`) にフォールバックします。データ自体は移動先に残っているので失われませんが、ランチャーが認識しません。**Settings → Runtime Path** で移動先フォルダーを再指定すれば復旧できます。フォルダーに既に GDLauncher のデータがあるので、ランチャーは「switch only」操作として扱い、ファイルコピーなしでマーカーだけ更新します。

## データベースを共有しない理由

Runtime Path にある `gdl_conf.db` には、アカウントトークン、Microsoft リフレッシュトークン、GDL アカウント状態、インスタンスメタデータが入ります。マシンローカルで機微情報を含むため、**誰かに渡してはいけません**。また 2 台の PC で同じデータベースを使うと、トークンリフレッシュで競合して両方がログアウト状態になります。

別の PC で同じインスタンスが欲しい場合は、`instances/` 配下のインスタンスフォルダーを手動でコピーするか、Cloud Instance Share 機能を使ってください。
