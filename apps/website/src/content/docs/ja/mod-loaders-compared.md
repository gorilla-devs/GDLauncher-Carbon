---
title: "Mod ローダー比較: Forge / NeoForge / Fabric / Quilt"
description: "GDLauncher は 4 つの Minecraft Mod ローダーをサポート。それぞれの位置づけ、互換性、Mod や Modpack によってどれを選ぶか。"
faq:
  - question: "Minecraft でどの Mod ローダーを使えばいい？"
    answer: "入れたい Mod や Modpack が要求するもの、これでほぼ決まります。自由に選ぶなら: 新しめバージョンの性能/QoL 系は Fabric、新しいコンテンツ大型 Mod は NeoForge、古い Modpack や最大の蓄積ライブラリは Forge。"
  - question: "Forge Mod は Fabric で動く？"
    answer: "動きません。Forge と Fabric の Mod は互換性なし。一方向けに書かれた Mod はもう一方ではロードされません。多くの人気 Mod は別々のビルドを提供しているので、Mod のページでサポート対象ローダーを確認してください。"
  - question: "NeoForge は Forge の置き換え？"
    answer: "新しい Minecraft バージョンでは実質的にそうです。NeoForge は 2023 年に Forge と同じ API を持つフォークとして始まりましたが、その後両者は分岐しているため、現在の Mod は通常両方で動かすのではなく NeoForge ビルドを別途公開します。1.20.4 以降の多くの Forge 系 Mod は NeoForge ビルドに移行。1.20.1 以前は依然として Forge が標準。"
  - question: "Fabric Mod は Quilt で動く？"
    answer: "ほぼ動きます。Quilt は Fabric のフォークで Fabric Mod を直接実行可能。一部の Quilt 専用 Mod は Quilt API を使うため Fabric では動きません。手持ちの Mod が全て Fabric ならどちらのローダーでも問題なし。"
  - question: "2 つのローダーを同居できる？"
    answer: "1 つのインスタンス内ではできません。インスタンスごとにローダーは 1 つ。両方使いたい場合は 2 つのインスタンスを作成。GDLauncher のインスタンスシステムはまさにこの用途向け、Forge と Fabric のインスタンスをクリック 1 つで切り替えられます。"
---

# Mod ローダー比較: Forge / NeoForge / Fabric / Quilt

## GDLauncher がサポートする 4 つの Mod ローダー

GDLauncher は Minecraft Java Edition の主要 Mod ローダー 4 種に加え、Vanilla (ローダーなし) もインストール/実行できます。カスタムインスタンスを作成するとき選びます。Modpack をインストールする場合はパックのマニフェストが指定したローダーになります。

### Forge

最古の Mod ローダー (2011 年開始)。Forge は最大規模のヒストリカル Mod ライブラリを持ち、特にコンテンツ系 Mod (Tinkers' Construct、Twilight Forest、Create の初期版など) で強い。多くの古い Modpack も Forge をターゲット。

更新は Fabric より遅め。新 MC バージョン対応は数週間~数か月後になることも。

### NeoForge

2023 年に Forge コミュニティの分裂で生まれた Forge フォーク。Forge と API スタイル (Mod は概ねソース互換) を保ちつつ、より高頻度でリリースされ、Forge Mod 開発の多くがこちらに移行しています。

MC 1.20.4 以降では 2 つのうち NeoForge の方が活発。多くの大型 Mod が Forge と同等、または Forge の代わりに NeoForge ビルドを提供するようになりました。

### Fabric

別の設計思想: 小さく、速く、モジュラー。新 MC バージョンのリリース直後 (時に数時間以内) に対応版が出ます。Mod エコシステムは性能系 (Sodium、Lithium、FerriteCore)、QoL (Mod Menu、Iris)、新時代の高品質コンテンツ Mod に強い。

性能優先や最新 MC バージョンを楽しみたいなら Fabric。

### Quilt

2022 年に Fabric から分裂。ガバナンスが異なり、追加 API があります。Quilt は Fabric Mod を直接実行できるため、実用上の違いは小さい: 特定の Mod が要求すれば Quilt、そうでなければ Fabric で同じ結果。

Quilt は Fabric より小さい専用エコシステムを持ちつつ、Fabric コンテンツとほぼ完全互換。

## 互換性マトリクス

| ビルド先 | Forge で動く | NeoForge | Fabric | Quilt |
|---|---|---|---|---|
| Forge | はい | 場合による (初期の NeoForge は新しいフォークだったため Forge Mod をそのまま動かせましたが、その後 API が分岐しているため、現在の Forge Mod の多くは NeoForge ビルドが必要) | いいえ | いいえ |
| NeoForge | いいえ | はい | いいえ | いいえ |
| Fabric | いいえ | いいえ | はい | はい |
| Quilt | いいえ | いいえ | Quilt-API 使用 Mod は不可、それ以外は可 | はい |

クロスローダーブリッジは実用本流に存在しません。`mods/` に置く JAR はインスタンスのローダーに一致する必要があります。

## 新規インスタンスでの選び方

通常、Mod や Modpack が選びます:

- **CurseForge / Modrinth から Modpack を入れる？** GDLauncher がパックマニフェストを読んで指定ローダーを入れます。選択の余地なし。
- **特定の 1 Mod を中心にカスタムインスタンスを組む？** Mod のページで指定を確認。「Fabric 1.21.x」とあれば Fabric 1.21.x インスタンスを作成。
- **複数 Mod のリストで組む？** Mod ごとに対応ローダーを調べ、共通項を選択。性能系 Mod は Fabric が多く、コンテンツ系は Forge/NeoForge が多い。

何の制約もない場合のおすすめ: 性能/見た目重視は **Fabric**、コンテンツ系のサバイバルは **NeoForge**。

## 既存インスタンスのローダー切替

GDLauncher は作成後でもローダー変更を許可します。詳しくは [How to Switch Mod Loaders on an Existing Instance](/guides/switch-mod-loader)。要点: インスタンス右クリック → Edit → 別のローダーを選ぶ。mods フォルダーはクリアされないため、旧ローダーの JAR が残ります。起動前に互換性のないものを手動で削除してください。

## ローダーバージョンについて

各ローダーは Minecraft とは独立したバージョン体系を持ちます。「Forge」を選ぶと Forge バージョン (MC 1.20.1 なら `47.2.0` など) も選択。多くの Mod は「期待される major と同じ」であれば動きますが、最小ローダービルドを要求する Mod もあります。CurseForge / Modrinth のページで確認してください。
