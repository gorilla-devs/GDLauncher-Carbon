---
title: "インスタンスフォルダーの構造"
description: "GDLauncher のインスタンスフォルダーの中身。Mod、ワールド、設定、スクリーンショット、ログがどこにあり、手動で削除して安全なものはどれか。"
faq:
  - question: "Minecraft のワールドはどこに保存される？"
    answer: "各ワールドは <runtime_path>/instances/<instance>/instance/saves/<world-name>/ にあります。saves/ フォルダーは最初にワールドを生成またはインポートしたときに作成されます。インスタンスを右クリック → Open Folder でフォルダーを開き、instance/saves に入ってください。バックアップはワールドフォルダーごとコピー。"
  - question: "クラッシュレポートはどこ？"
    answer: "インスタンス内の instance/crash-reports/。フォルダーは Minecraft が実際にクラッシュしたときにのみ存在、未経験ならまだありません。タイムスタンプ付きテキストファイル (crash-<date>-server.txt など)。"
  - question: "Mod JAR はどこに置く？"
    answer: "インスタンス内の instance/mods/。手動で JAR を入れても動きますが GDLauncher の管理外。理由がなければ Addons タブを使用。ロックされた Modpack インスタンスは Addons の Add ボタンが無効ですが手動ファイルコピーは通ります。"
  - question: "ランチャーのログと Minecraft のログの違いは？"
    answer: "2 種類あります。ランチャー側のセッションログは <instance>/logs/。Minecraft 自体のゲームログ (latest.log、crash-reports、ゲームが書き出すすべて) は 1 階層下の <instance>/instance/logs/ と <instance>/instance/crash-reports/。"
  - question: "instance.json と packinfo.json とは？"
    answer: "GDLauncher がインスタンスのトップレベルに書くメタデータファイル。instance.json はインスタンス名、アイコン、Mod ローダー、Minecraft バージョン、最終プレイ時刻、プレイ時間を保持。packinfo.json (CurseForge / Modrinth Modpack とペアのインスタンスでのみ存在) はパックに属するファイルを追跡し、パック管理 Mod とユーザー追加 Mod を区別。どちらも手動削除しないでください。"
---

# インスタンスフォルダーの構造

## インスタンスフォルダーの場所

各 GDLauncher インスタンスは Runtime Path のサブフォルダーに置かれます:

```
<runtime_path>/
└── instances/
    └── <shortpath>/        ← インスタンスフォルダー
```

`<shortpath>` は表示名をサニタイズしたもの。GDLauncher 内でインスタンスを右クリック → **Open Folder** で直接開けます。

## 中身

インスタンスフォルダーはトップレベルに GDLauncher が追跡するいくつかの項目と、Minecraft の実際のゲームディレクトリとなる `instance/` サブフォルダーに分かれます。常に存在するものもあれば、何かが書き込もうとした最初のときに作成されるものもあります。

```
<shortpath>/
├── instance.json          ← GDLauncher のインスタンスメタデータ (常に存在)
├── packinfo.json          ← Modpack ペアリング情報 (ペア済みパックのみ)
├── icon.png | icon.webp   ← カスタムアイコン (設定時のみ)
├── logs/                  ← GDLauncher のインスタンス別ランチャーログ
└── instance/              ← Minecraft のゲームディレクトリ
    ├── mods/              ← Mod JAR
    ├── config/            ← Mod 設定
    ├── shaderpacks/       ← シェーダーパック (インストール時)
    ├── options.txt        ← Minecraft クライアント設定 (初回起動後)
    ├── logs/              ← Minecraft セッションログ (latest.log など)
    ├── saves/             ← ワールド (ワールド生成時に作成)
    ├── screenshots/       ← F2 スクリーンショット (初回 F2 時に作成)
    ├── crash-reports/     ← クラッシュダンプ (クラッシュが起きたときのみ)
    ├── resourcepacks/     ← カスタムリソースパック (追加時)
    ├── datapacks/         ← グローバルデータパック (ワールド限定は saves/<world>/datapacks/)
    └── (パック固有)        ← kubejs/、defaultconfigs/、packmenu/ など、パックが同梱する場合のみ
```

新しいインスタンスでこれらの一部がないことを気にしないでください。ランチャーと Minecraft は必要なときに必要なものだけ作成します。一度もプレイしていないインスタンスには `saves/`、`screenshots/`、`options.txt` はありません。Vanilla インスタンスには `mods/` も `config/` もありません。

## 各項目の中身

### トップレベルファイル

- **`instance.json`**: GDLauncher のメタデータ、名前、アイコンパス、Mod ローダーとバージョン、Minecraft バージョン、作成時刻、最終プレイ時刻、合計プレイ時間。常に存在。
- **`packinfo.json`**: Modpack から来たファイルのハッシュマニフェスト。パック管理 Mod と自分で追加したものをランチャーが区別できます。CurseForge / Modrinth Modpack とペアのインスタンスでのみ存在。
- **`icon.png`** または **`icon.webp`**: アップロードしたカスタムアイコン。デフォルトアイコンの場合は存在しません。

### `logs/` (トップレベル)

GDLauncher のインスタンス別ログ。右クリック → **View Logs** で開けるのはここの中身。*ランチャー側* から見た起動 (Java 引数、アセットダウンロード、Mod ローダーインストール、終了コード) を記録。Minecraft 本体が自前のログを書く前にクラッシュしたときに役立ちます。

### `instance/mods/`

Mod の JAR ファイル。Minecraft が起動時にここの全部を読み込み (Mod ローダーのルールに従う)。ランチャーはどの Mod がインスタンスに属するかを内部 DB で管理 (ファイル名とコンテンツハッシュをキーに), サイドカーファイルはありません。手動で置いた JAR も拾われますが、CurseForge / Modrinth メタデータはランチャー側にありません。

### `instance/config/`

Mod ごとに 1 サブフォルダーまたは 1 ファイル。Mod 設定の永続化先。多くの Mod は `config/<modid>.toml` や `config/<modid>/` を作ります。手編集は基本的に安全で、再起動時に反映されます。

### `instance/resourcepacks/`, `instance/shaderpacks/`, `instance/datapacks/`

アセットパック。リソースパックはテクスチャと音、シェーダーパックは描画 (Iris/OptiFine などを Mod として導入していること前提)、データパックはレシピ・ルートテーブル・ファンクションを追加。ワールド限定のデータパックは `saves/<world>/datapacks/` に入れます。これらのフォルダーは実際にコンテンツがあるときのみ作成されます。

### `instance/saves/`

ワールドごとに 1 サブフォルダー。中の `level.dat` がマスターファイル、`region/` にチャンクデータ、`playerdata/` にプレイヤー別状態、`datapacks/` にワールドスコープのデータパック。ワールドのバックアップは `<world>/` フォルダーをまるごとコピー。`saves/` 自体は初めてワールドを生成したときに作成されます。

### `instance/screenshots/`

F2 で撮った PNG、タイムスタンプで命名。初回スクリーンショット時に作成されます。

### `instance/logs/` と `instance/crash-reports/`

Minecraft 自身の診断出力。`logs/latest.log` は常に最新の起動分 (次の起動時に `logs/<date>-1.log.gz` にロール)。`crash-reports/` は完全なクラッシュダンプ、実際にクラッシュが発生したときにのみ作成されます。

### `instance/options.txt`

Minecraft クライアント設定 (グラフィック、操作、サウンド)。プレーンテキスト、key=value。手編集可能。

### パック固有のフォルダー

大型 Modpack は追加フォルダーを同梱します。よくあるもの:

- **`kubejs/`**: KubeJS スクリプト (`server_scripts/`、`client_scripts/`、`startup_scripts/`、`data/`、`assets/`)。パック作者がランタイム調整に使用。
- **`defaultconfigs/`**: 「デフォルトでの設定の見た目」のスナップショット。パックの起動スクリプトが各起動で `config/` に欠落エントリをコピー。
- **`packmenu/`**: パックテーマのメインメニューアセット (カスタムボタン、背景、スプラッシュテキスト)。
- **`defaultsettings/`**: `defaultconfigs/` と類似だが、`options.txt` とキーバインド用。

パックが同梱する場合のみ存在。Vanilla とほとんどのカスタムインスタンスにはありません。

## 削除して安全なもの

| フォルダー | 安全？ | 影響 |
|---|---|---|
| `instance/mods/` の特定 JAR | はい | その Mod は消える。それを使うワールドは壊れる可能性あり。 |
| `instance/config/<modid>/` | はい | 次回起動時にデフォルトへ戻る。 |
| `instance/resourcepacks/`、`instance/shaderpacks/` の内容 | はい | パックが消える。 |
| `instance/saves/<world>/` | はい | ワールドが永久に消える。先にバックアップを。 |
| `instance/logs/`、`crash-reports/` | はい | 容量解放のみ。 |
| `instance/screenshots/` | はい | 古いスクリーンショットさよなら。 |
| `logs/` (ランチャーログ) | はい | 同上。 |
| `instance/options.txt` | はい | ゲーム設定がデフォルトに戻る。 |
| `instance.json` | だめ | ランチャーがインスタンスを追跡できなくなる。 |
| `packinfo.json` | 可能 (実質アンペア) | ランチャーがペア済み Modpack として扱わなくなる。インスタンスの Settings タブの Unpair ボタンと同じ効果だが、こちらは雑。 |
| `instance/` フォルダーごと | だめ | インスタンスが壊れる。ランチャー UI の Delete を使用。 |

## ロックされた Modpack インスタンスについて

インスタンスフォルダーはディスク上の普通のフォルダー。GDLauncher がかける Modpack ロックは UI レベルで強制されるもので、ファイルシステムでは強制されません。ロックされたインスタンスの `instance/mods/` に直接 JAR を入れれば動きますが、Addons タブからは認識されません。削除はファイルシステムから行う必要があります。安全な運用のため、まずインスタンスを開いて **Settings** タブの Modpack セクションで **Unlock** を押すのを推奨。
