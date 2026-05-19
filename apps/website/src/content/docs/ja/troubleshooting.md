---
title: "トラブルシューティング"
description: "GDLauncher と Minecraft の起動に関するよくあるトラブルを解決します。アプリのデータパス、ランタイムパス、ログの場所、確実な解決策をまとめました。"
faq:
  - question: "GDLauncher のデータはどこに保存されますか？"
    answer: "Windows: C:\\Users\\<ユーザー名>\\AppData\\Roaming\\gdlauncher_carbon。macOS: /Users/<ユーザー名>/Library/Application Support/gdlauncher_carbon。Linux: $XDG_DATA_HOME/gdlauncher_carbon（XDG が未設定の場合は ~/.local/share/gdlauncher_carbon）。"
  - question: "GDLauncher のログはどこにありますか？"
    answer: "GDLauncher はアプリレベルのログを 2 つのファイルに書き込みます: アプリデータフォルダーの main.log (Electron) と、ランタイムパスの __gdl_logs__ フォルダー内のタイムスタンプ付き <timestamp>.log (Rust コア、最新 10 件保持)。問題を報告する際は両方とも送付してください。詳細な場所は Share App Logs ガイドを参照。"
  - question: "GDLauncher が起動しません。どうすればいいですか？"
    answer: "まずデータフォルダーのログを確認し、エラーを探してください。よくある原因は、ランタイムの破損、ウイルス対策ソフトによる実行ファイルのブロック、適用が中途半端なアップデートなどです。GDLauncher をクリーンインストールしてインスタンスを復元すると、たいてい解消します。"
  - question: "Modpack が起動時にクラッシュするのはなぜですか？"
    answer: "起動時クラッシュの多くは、Minecraft のバージョン・Mod ローダー・Mod の組み合わせが合っていないことが原因です。__gdl_logs__ フォルダー内の最新ログでエラーを確認してください。特定の Mod 名が示されている場合はそれが原因のことが多いので、Addons タブで無効化してから再起動してください。OutOfMemoryError の場合は、インスタンス設定で割り当て RAM を増やしてください。"
  - question: "GDLauncher を別のドライブやフォルダーへ移動するには？"
    answer: "設定 → 一般 → ランタイムパスを開きます。新しい場所に変更すると、GDLauncher がインスタンスとダウンロードを自動で移行します。移行は次回起動時に一度だけ実行されます。"
  - question: "GDLauncher はオフラインでも使えますか？"
    answer: "すでにインストール済みのインスタンスはオフラインでもプレイできます。ただし認証には少なくとも一度オンラインでのサインインが必要（Microsoft アカウント）で、新しい Mod や Modpack のダウンロードにはインターネット接続が必要です。"
---

## アプリのデータパス

GDLauncher が Electron のデータを保存する場所であり、デフォルトでは Core Module のランタイムパスもここに置かれます。

### Windows

`C:\Users\\{{ユーザー名}}\\AppData\Roaming\gdlauncher_carbon`

### macOS

`/Users/{{ユーザー名}}/Library/Application Support/gdlauncher_carbon`

### Linux

- 環境変数 `$XDG_DATA_HOME` が設定されている場合: `$XDG_DATA_HOME/gdlauncher_carbon`
- それ以外: `{{homedir}}/.local/share/gdlauncher_carbon`

[homedir の詳細はこちら](https://nodejs.org/api/os.html#oshomedir)

## Core Module ランタイムパス

Core Module がすべてのデータ（インスタンス、アセット、ライブラリなど）を保存する場所です。
明示的に別の場所を指定しない限り、アプリのデータパスと同じ場所の `data` フォルダーに置かれます。

### アプリデータベース

アプリデータベースは Core Module のランタイムパス内に置かれており、`gdl_conf.db` という名前の SQLite データベースファイルです。

**このファイルには機密情報が含まれているため、絶対に他人に送らないでください。**

### アプリログ

GDLauncher はアプリレベルのログを 2 つのファイルに書き込みます。サポートには **常に両方** を送ってください。ランチャーの 2 つのプロセスは互いに作業を引き継ぐため、一方の障害の原因がもう一方のログに現れることが多いためです。

- **`main.log`** (App Data Path 内): Electron メインプロセスのログ。ウィンドウ生成、IPC、自動更新、ネイティブダイアログ、デスクトップシェルのハードクラッシュなどを記録。
- **`__gdl_logs__/<timestamp>.log`** (Core Module Runtime Path 内): Rust コアのログ。アカウントサインイン、アセットダウンロード、Mod ローダーのインストール、インスタンス起動、設定変更などを記録。最新 10 件を保持。

OS 別のパスやスクリーンショットは [Share App Logs](/guides/share-app-logs) を参照。

**ログには機密情報が含まれることがあります。共有時はご注意ください。**

### ランタイムパスの変更

ランタイムパスを変更すると、アプリはすべてのインスタンスと設定ファイルを自動的に新しい場所へ移動します。

移行先のフォルダーがすでに使用されている場合、アプリはランタイムパス設定を切り替えるだけで、ファイルの移動やコピーは行いません。

#### 移行エラー

移行に失敗すると、アプリはエラーメッセージを表示します。

まずはエラーメッセージの内容を確認してください。
ファイルのコピー自体は成功していて、古いファイルの削除で失敗しているケースが多くあります。その場合はアプリを閉じてから、古いファイルを手動で削除してください。

旧ランタイムパス内にある `runtime_path_override` というファイルは、ランタイムパスが変更されたかどうかをアプリが検知するために使われるので、削除しないでください。

ご不明な点があれば、[Discord サーバー](https://discord.gdlauncher.com) に参加してご質問ください。
