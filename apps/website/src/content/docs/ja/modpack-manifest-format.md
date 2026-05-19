---
title: "Modpack マニフェスト形式"
description: "GDLauncher が読み書きする 3 種類の Modpack アーカイブの中身: CurseForge .zip、Modrinth .mrpack、そして GDLauncher 独自のハッシュベース .gdlpack。フィールド別リファレンス、横並び比較、各形式がインストール時にどう Mod を解決するかの解説。"
faq:
  - question: "Modpack マニフェストとは？"
    answer: "Modpack アーカイブ内にある JSON ファイルで、パックの内容 (Minecraft バージョン、Mod ローダー、Mod 一覧、インスタンスに展開する同梱ファイル (オーバーライド) 一覧) をランチャーに伝えるもの。GDLauncher はパックをインストールする前にまずマニフェストを読みます。"
  - question: "CurseForge、Modrinth、GDLauncher の各パック形式の違いは？"
    answer: "3 つとも JSON マニフェストを内包する zip。CurseForge は manifest.json を使い CurseForge のプロジェクト / ファイル ID で参照。Modrinth は modrinth.index.json を使い直接ダウンロード URL とハッシュで参照。GDLauncher は gdlpack.json を使いコンテンツハッシュのみ (sha512 + sha1 + murmur2) で、各ファイルを実際に存在するプラットフォームに対して解決します。互換性はなく、ランチャーはアーカイブ直下にある JSON によって処理を分けます。"
  - question: ".gdlpack とは何で、なぜ存在するのか？"
    answer: "GDLauncher 独自の Modpack 形式。各 Mod はコンテンツハッシュで識別されるので、Mod が CurseForge にあっても Modrinth にあっても両方にあっても、同じパックが動作し、URL が腐ることもありません。オプション機能 (スキップ可能な Mod + 設定のバンドル) や埋め込みアイコンも一級サポート。送り手と受け手の両方が GDLauncher を使う場合に推奨。クロスランチャー配布には CurseForge .zip の方が無難です。"
  - question: "インストール済みインスタンスでマニフェストを見るには？"
    answer: "GDLauncher はインストール後、元のアーカイブを残しません。パック内容はインスタンスフォルダーに展開済み。GDLauncher が参照するマニフェストはランチャーのデータベースに紐付いており、ファイルとしては閲覧できません。インストール前に確認したいなら、.zip / .mrpack / .gdlpack を任意のアーカイブツールで開いて中の JSON ファイルを直接読んでください。"
  - question: "マニフェストを編集できる？"
    answer: "自分でパックを作る場合は .zip 内のものを編集できます。インストール済みインスタンスのマニフェストはランチャー UI から編集できません。Mod を変えるには Addons タブを使用 (またはロックされたインスタンスを先にアンロック)。ペア済みパックのバージョンを変えるには、インスタンスを開いて Settings タブの Change Modpack Version をクリック。"
  - question: "パック用語の「override」とは？"
    answer: "Mod ではなく、パックが事前同梱するファイル。デフォルト設定、スクリプト、リソースパック、時にはスタートワールドなど。パックインストール時に Mod の上にインスタンスフォルダーへ展開されます。CurseForge は overrides フィールドで列挙、Modrinth は env.client=required のパックファイルを使用、GDLPack は overrides フォルダー (マニフェストで設定可能) を使います。"
---

# Modpack マニフェスト形式

## なぜ知る必要があるか

通常 Modpack アーカイブの中身を知る必要はありません。しかし、インストール中にトラブルがあると「manifest」「invalid manifest format」「manifest missing field」「manifest version unsupported」といったメッセージが表示され、それらが何を指すか知りたくなります。共有用にパックを作るとき、あるいは同じパックが片方のランチャーではきれいに入って別のランチャーでは壊れる原因を探るときにも有用です。

GDLauncher は 3 種類の形式を読み書きします:

- **CurseForge** (`.zip`、内部に `manifest.json`)。
- **Modrinth** (`.mrpack`、内部に `modrinth.index.json`)。
- **GDLauncher** (`.gdlpack`、内部に `gdlpack.json`)。

3 つとも内部はプレーンな ZIP アーカイブ。拡張子はどのマニフェストスキーマを期待すべきかの目印にすぎません。

## Modpack アーカイブの中身

3 形式とも全体的な構造は同じ:

```
mypack.{zip,mrpack,gdlpack}/
├── <manifest>.json       ← 形式ごとのマニフェスト
├── overrides/            ← 同梱の設定、スクリプト、任意のスタートワールド
│   ├── config/
│   ├── scripts/
│   └── ...
└── modlist.html          ← (CurseForge のみ、任意、人間用 Mod リスト)
```

マニフェストがランチャーのパックインストールに必須な唯一のファイル。overrides フォルダーは出来上がるインスタンスに展開されるコンテンツ。

## CurseForge: `manifest.json`

```json
{
  "minecraft": {
    "version": "1.20.1",
    "modLoaders": [{ "id": "forge-47.2.0", "primary": true }]
  },
  "manifestType": "minecraftModpack",
  "manifestVersion": 1,
  "name": "Example Pack",
  "version": "1.0.0",
  "author": "someone",
  "files": [
    { "projectID": 238222, "fileID": 5246076, "required": true }
  ],
  "overrides": "overrides"
}
```

主要フィールド:

- `minecraft.version`: パックがターゲットとする Minecraft バージョン。
- `minecraft.modLoaders`: どのローダーとどのバージョン (フォーマットは `<loader>-<version>`)。
- `files`: 各 Mod は CurseForge `projectID` と `fileID` で参照。GDLauncher がこれらを CurseForge API で解決してダウンロード。
- `overrides`: zip 内のフォルダー名で、Mod ダウンロード完了後にその中身がインスタンスへコピーされる。

`projectID` か `fileID` の Mod が CurseForge 上から消えている (作者削除) と、その Mod について「file not found」エラーでインストール失敗。

## Modrinth: `modrinth.index.json`

```json
{
  "formatVersion": 1,
  "game": "minecraft",
  "versionId": "1.0.0",
  "name": "Example Pack",
  "dependencies": {
    "minecraft": "1.21.1",
    "fabric-loader": "0.16.5"
  },
  "files": [
    {
      "path": "mods/sodium-fabric-0.6.0.jar",
      "hashes": { "sha1": "...", "sha512": "..." },
      "env": { "client": "required", "server": "unsupported" },
      "downloads": [
        "https://cdn.modrinth.com/data/AANobbMI/versions/.../sodium.jar"
      ],
      "fileSize": 1234567
    }
  ]
}
```

CurseForge との主な違い:

- `dependencies` で Minecraft とローダーバージョンを直接宣言 (ネストオブジェクトなし)。
- 各 `files` エントリに **正確なダウンロード URL** と **ハッシュ** が含まれ、メタデータ API 呼び出しなしでインストール可能。
- 各ファイルがクライアント/サーバー/両方のどこで必須かを `env` で宣言。

形式がシンプルでオフライン対応しやすい (URL が埋め込まれている)。より厳格で、ハッシュ不一致は警告ではなくインストール拒否。

## GDLauncher: `gdlpack.json`

GDLauncher 独自のパック形式は、**Export Instance** で **GDLauncher .gdlpack** を選んだときに GDLauncher が書き出す形式。決定的な特徴: 各 Mod はプラットフォーム固有 ID や URL ではなく、コンテンツハッシュだけで識別されます。GDLauncher がインストール時に各ハッシュを CurseForge と Modrinth に対して解決し、ファイルが実際にあるプラットフォームからダウンロードします。

### なぜ ID や URL ではなくハッシュなのか

- **1 つのソースでクロスプラットフォーム。** CurseForge と Modrinth 両方にある Mod は、各サイトで ID が異なる。`.gdlpack` は気にしない。1 つのハッシュリストで両方に対応。
- **URL 腐敗なし。** Modrinth CDN URL はコンテンツアドレス指定で安定だが、URL を埋め込むとダウンロード元が固定される。ハッシュなら片方のプラットフォームが落ちていてもフォールバック可能。
- **構造的な検証。** `mods/` に書き込まれる全バイトがマニフェストのハッシュと照合される。別の JAR をこっそり差し替える手段はありません。

### アーカイブ構造

```
mypack.gdlpack/
├── gdlpack.json          ← マニフェスト
├── .gdl/
│   └── icon.png          ← 埋め込みパックアイコン (任意)
└── overrides/            ← 同梱ファイル (設定、スクリプト、解決不能 Mod)
    ├── config/
    └── ...
```

マニフェストは直下。`.gdl/` フォルダーにはランチャーが埋め込むメタデータ (現状はアイコンのみ)。overrides は CurseForge と同じで、Mod 解決後の出来上がりインスタンスに展開されます。

### 最小マニフェスト

```json
{
  "formatVersion": 1,
  "name": "Example Pack",
  "createdAt": "2026-05-11T10:30:00Z",
  "dependencies": {
    "minecraft": "1.20.1",
    "modloaders": [
      { "type": "forge", "version": "47.2.0", "primary": true }
    ]
  },
  "entries": [],
  "overrides": "overrides"
}
```

ロードに必須なのは `formatVersion`、`name`、`createdAt`、`dependencies.minecraft`、`entries`、`overrides` のみ。

### フルマニフェスト (注釈付き)

```json
{
  "formatVersion": 1,
  "name": "Example Pack",
  "version": "1.0.0",
  "summary": "A short pack tagline",
  "author": "Pack Author",
  "createdAt": "2026-05-11T10:30:00Z",
  "icon": ".gdl/icon.png",
  "dependencies": {
    "minecraft": "1.20.1",
    "modloaders": [
      { "type": "forge", "version": "47.2.0", "primary": true }
    ]
  },
  "entries": [
    {
      "type": "platform",
      "hashes": {
        "sha512": "abc...",
        "sha1": "def...",
        "murmur2": 123456789
      }
    },
    {
      "type": "optional",
      "description": "Shader support - skip for low-end GPUs",
      "platforms": [
        { "sha512": "xyz...", "sha1": "uvw...", "murmur2": 987654321 }
      ],
      "overridePaths": [
        "config/iris",
        "shaderpacks/default"
      ]
    },
    {
      "type": "optional",
      "description": "Hardcore difficulty preset",
      "overridePaths": ["config/hardcore"]
    }
  ],
  "overrides": "overrides",
  "serverOverrides": null,
  "clientOverrides": null,
  "source": {
    "platform": "curseforge",
    "projectId": 12345,
    "fileId": 67890,
    "name": "Original Pack",
    "url": null
  }
}
```

#### トップレベルフィールド

| フィールド | 型 | 必須 | 意味 |
|---|---|---|---|
| `formatVersion` | integer | はい | マニフェストスキーマバージョン。現状は常に `1`。 |
| `name` | string | はい | パック表示名。インポート時のデフォルトインスタンス名。 |
| `version` | string | いいえ | パックバージョン (semver 推奨、例: `1.0.0`、`2.1.0-beta.1`)。 |
| `summary` | string | いいえ | 一行のキャッチフレーズ。 |
| `author` | string | いいえ | パック作者またはチーム。 |
| `createdAt` | RFC 3339 タイムスタンプ | はい | アーカイブをエクスポートした日時。 |
| `icon` | string | いいえ | アーカイブ内のアイコンファイルへのパス (例: `.gdl/icon.png`)。 |
| `dependencies` | object | はい | Minecraft と Mod ローダーの要件 (下記参照)。 |
| `entries` | array | はい | プラットフォームファイルとオプション機能 (下記参照)。overrides のみのパックなら空配列も有効。 |
| `overrides` | string | はい (デフォルトは `"overrides"`) | インスタンスへ展開するアーカイブ内ディレクトリ名。 |
| `serverOverrides` | string | いいえ | サーバー専用 overrides のディレクトリ。 |
| `clientOverrides` | string | いいえ | クライアント専用 overrides のディレクトリ。 |
| `source` | object | いいえ | CurseForge / Modrinth パックから派生した場合、元パックを指す (下記参照)。 |

#### `dependencies`

```json
{
  "minecraft": "1.20.1",
  "modloaders": [
    { "type": "forge", "version": "47.2.0", "primary": true }
  ]
}
```

- `minecraft`: 必須の Minecraft バージョン (例: `1.20.1`、`1.21.4`、スナップショットなら `25w20a`)。
- `modloaders`: 0 個以上のローダーエントリ。各エントリ:
  - `type`: `forge`、`neoforge`、`fabric`、`quilt` のいずれか (小文字)。
  - `version`: パックがビルドされた正確なローダーバージョン。
  - `primary`: 複数列挙時の推奨ローダーを示す (例: Fabric と Quilt の両方に対応するパック)。

バニラパックは `modloaders` が空配列。

#### `entries`

`type` フィールドで識別される 2 種類のエントリ:

**`platform`**, ハッシュ経由で解決される必須 Mod:

```json
{
  "type": "platform",
  "hashes": {
    "sha512": "...",
    "sha1": "...",
    "murmur2": 1234567890
  }
}
```

3 つのハッシュすべてが必須。解決時にそれぞれ用途が異なります:

| ハッシュ | 用途 |
|---|---|
| `sha512` | Modrinth 解決 (Modrinth の `version_file` エンドポイント)、ダウンロードファイルの主要整合性チェック。 |
| `sha1` | Modrinth 解決 (フォールバック)、起動時に Minecraft 自身がファイル検証で使用。 |
| `murmur2` | CurseForge 解決 (CurseForge の `fingerprints` エンドポイント)。 |

インストール時、ランチャーはまず Modrinth (sha512 ルックアップ) を試し、CurseForge (murmur2 フィンガープリント) にフォールバック、両方になければ当該エントリを失敗とします。解決されたファイルは常に応答したプラットフォームからダウンロードされます。

**`optional`**, ユーザーが含めるか除外するか選べる、スキップ可能な Mod / オーバーライドパスのグループ:

```json
{
  "type": "optional",
  "description": "Shader support - skip for low-end GPUs",
  "platforms": [
    { "sha512": "...", "sha1": "...", "murmur2": 1234567890 }
  ],
  "overridePaths": [
    "config/iris",
    "shaderpacks/default"
  ]
}
```

- `description`: インポートプレビューでユーザーに表示され、機能を含めるか判断する材料。
- `platforms`: この機能を含めたときだけダウンロードされる 0 個以上のハッシュエントリ。
- `overridePaths`: この機能を含めたときだけ展開される `overrides/` 内の 0 個以上のパス (ファイルかフォルダー)。パスは `overrides/` からの相対。

機能はプラットフォームファイルだけ、overrides だけ、両方、いずれも可能。関連する任意コンテンツのまとめ方として使えます: シェーダー Mod とその設定、設定のみのハードコア難易度プリセット、任意のリソースパックなど。

#### `source`

既存の CurseForge / Modrinth Modpack インスタンスからエクスポートされた場合、GDLauncher は出処を記録します:

```json
{
  "platform": "curseforge",
  "projectId": 12345,
  "fileId": 67890,
  "name": "Original Pack",
  "url": null
}
```

または

```json
{
  "platform": "modrinth",
  "projectId": "AANobbMI",
  "versionId": "abc123",
  "name": "Original Pack",
  "url": null
}
```

情報用で、ランチャーはこれを使ってダウンロードはしません (それは `entries` 配列の役目)。公開パックの派生物が元パックへのアトリビューションを保つために存在します。

### バンドル vs マニフェストのみエクスポート

`.gdlpack` にエクスポートするとき、エクスポートウィザードの **Bundle Addons** トグルで内容物の配置が決まります:

- **バンドル OFF (マニフェストのみ)。** GDLauncher が CurseForge / Modrinth で解決できる各 Mod は `entries` の `platform` エントリに。実際の JAR はアーカイブに *含まれません*。受け手のランチャーがインポート時にプラットフォームから再ダウンロード。GDLauncher が解決できない Mod (手動で投げ込んだ JAR など) はフォールバックとして `overrides/mods/` にバンドルされます。結果: 小さなアーカイブ、受け手はインストール時にネット必須。
- **バンドル ON (自己完結)。** `platform` エントリは出力されず、全 Mod が `overrides/mods/` にコピー。結果: 大きなアーカイブ (数百 MB から数 GB)、Minecraft アセットがキャッシュ済みならオフラインでインストール可能。

リソースパックとシェーダーパックも同じ振る舞い: バンドル OFF なら解決可能なものは `platform` エントリ、バンドル ON なら全て直接バンドル。

## 横並び比較: 3 形式

| プロパティ | CurseForge `.zip` | Modrinth `.mrpack` | GDLauncher `.gdlpack` |
|---|---|---|---|
| マニフェストファイル名 | `manifest.json` | `modrinth.index.json` | `gdlpack.json` |
| Mod 識別 | CurseForge プロジェクト + ファイル ID | ダウンロード URL + ハッシュ | コンテンツハッシュのみ |
| 解決元 | CurseForge API | マニフェスト内 URL | CurseForge **か** Modrinth (応答した方) |
| オフラインインストール | 不可 (CDN 必須) | 可能 (URL 自体は CDN 必須かも) | `Bundle Addons` ON なら可能 |
| オプション機能 | Mod ごとに `required: false` | ファイルごとに `env` (client/server) | 専用 `optional` エントリで複数ファイル可 |
| オーバーライドフォルダー | `overrides/` | ファイルごとのパス | `overrides/` (設定可) |
| サーバー専用ファイル | 別パック | `env.server` | `serverOverrides` ディレクトリ |
| アイコン | 外部 | 外部 | `.gdl/icon.<ext>` に埋め込み |
| ソース追跡 | なし | なし | `source` フィールド |
| 移植性 | 最広 (ほぼ全ランチャー対応) | Modrinth 対応ランチャー | GDLauncher |

## Overrides の詳細

マニフェストが overrides フォルダーや client-required としてマークされたパックファイルを参照すると、ランチャーは Mod 解決ステップ後にそれらをインスタンスへ展開します。パックはこれで以下を同梱できます:

- デフォルト Mod 設定 (誰がインストールしても同じ初期体験になる)。
- KubeJS / CraftTweaker スクリプト。
- スタートワールド (たまに)。
- パックが有効化を期待するリソースパック。
- パック向けに調整された `options.txt`。

Override は自動生成デフォルトより優先されますが、後でプレイヤーが手動で変えたものには負けます。GDLPack はさらに `serverOverrides` と `clientOverrides` ディレクトリをサポートし、片側だけに配置すべきファイル (一致するクライアントとサーバーのバンドルを配布するパック用) に便利です。

## マニフェストが古くなるとき

パックマニフェストはパック作者が最後に公開したスナップショット。マニフェスト構築後にソースプラットフォームから参照先 Mod が削除されると、そのパックバージョンのインストールは作者が新バージョンを公開するまで失敗します。古いパックバージョンはきれいに入るのに新しいバージョンが壊れるのはこのケース: 新バージョンが現在利用できないものを参照しています。

修正はパック作者側の作業で、ランチャーは与えられたマニフェスト通りにしか動けません。GDLPack はこの点で部分的な優位を持ちます: 単一プラットフォームに縛られないため、Mod が CurseForge から消えても Modrinth に残っていれば (逆も同様)、同じ `entries` 配列は依然解決します。

## 自分でマニフェストを読む

特別なツール不要です。`mypack.mrpack` や `mypack.gdlpack` を `mypack.zip` に改名、任意のアーカイブツールで開き、中の JSON ファイル (`modrinth.index.json` または `gdlpack.json`) を見るだけ。CurseForge `.zip` の `manifest.json` も同様。3 つともプレーン JSON、テキストエディタで読めます。

## パックを作る

共有用にパックを作る場合、最も簡単な道は GDLauncher でインスタンスを構築し、**Export Instance** (右クリック → Export Instance) を使うこと。エクスポートウィザードで出力形式を選べます: CurseForge `.zip`、Modrinth `.mrpack`、または GDLauncher 独自の `.gdlpack`。受け手も GDLauncher を使い、オプション機能や埋め込みアイコンが欲しいなら `.gdlpack`。クロスランチャー互換性を最大化したいなら `.zip`。
