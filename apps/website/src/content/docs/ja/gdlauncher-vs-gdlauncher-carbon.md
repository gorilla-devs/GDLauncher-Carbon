---
title: "GDLauncher と GDLauncher Carbon"
---

# GDLauncher と GDLauncher Carbon

GDLauncher Carbon は GDLauncher の新しいバージョンで、新デザインと新機能を備え、ゼロから作り直されました。

## なぜ新しいバージョンを作っているのですか？

新バージョンを作る理由はいくつもあります。多くは[こちら](/blog/curseforge-partnership-announcement)で読めます。

## 何が新しくなりましたか？

GDLauncher Carbon は、より効率的で信頼性の高い技術スタックでゼロから書き直されています。フロントエンドには Electron と SolidJS を採用し、重い処理は Rust で書かれた別バイナリ（Core Module）が担当します。

書き直しに合わせて UI / UX も全面的に刷新し、これまでの雰囲気を残しつつ、より使いやすくモダンなものになりました。

旧バージョンとの機能パリティはすでに達成・上回っており、現在は仕上げと新機能の追加に注力しています。

新機能の例:

- **新しい Java マネージャー**: GDLauncher に Java を任せることも、強化された管理機能で手動でバージョンを管理することもできます。
- **シームレスな Mod と Mod ローダーのインストール**: あらゆる Minecraft バージョンと、Forge、Fabric、Quilt、NeoForge などの Mod ローダーをこれまで以上に簡単にインストールできます。
- **充実した Addon／Modpack 対応**: CurseForge と Modrinth から Addon や Modpack を直接インストールできます。
- **Modpack アップデーター**: いつでも Modpack のバージョンを変更でき、適用された変更点の正確な変更履歴を生成します。
- **インスタンスのインポート／エクスポート**: （作業中）GDLauncher Carbon と CurseForge、MultiMC、ATLauncher、Technic、Prism、Modrinth、FTB などのランチャー間でゲームインスタンスを移動できるよう開発を進めています。

## 旧バージョンはどうなりますか？

旧バージョンは引き続きダウンロード可能ですが、メンテナンスは行いません。

最近、深刻な不具合が多数報告されているため、できるだけ早く GDLauncher Carbon に移行することをおすすめします。
