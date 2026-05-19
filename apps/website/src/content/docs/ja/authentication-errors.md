---
title: "Microsoft 認証エラー"
description: "GDLauncher で発生する Microsoft 認証エラーの解決方法。Invalid Grant、アカウント停止、コンソールアクセス必須、Xbox Live エラーへの対処法を解説します。"
faq:
  - question: "GDLauncher で 'Invalid Grant' エラーが出るのはなぜですか？"
    answer: "'Invalid Grant' エラーは、Microsoft アカウントのセキュリティに関する問題が原因であることが多いです。よくある対処法は、Microsoft アカウントの 2 段階認証を有効化する、パスワードが未設定なら設定する、いったんサインアウトして再度サインインする、などです。"
  - question: "GDLauncher にアカウントが BAN されていると表示されるのはなぜですか？"
    answer: "GDLauncher にアカウントの BAN が表示される場合、その BAN は Mojang または Microsoft によるものであり、GDLauncher 側のものではありません。minecraft.net または Microsoft アカウントにサインインして BAN の理由を確認してください。GDLauncher は認証応答を中継するだけで、独自の BAN リストはありません。"
  - question: "GDLauncher にコンソールアクセスが必要と表示されます。なぜですか？"
    answer: "これは通常、子ども用アカウントやファミリーグループによる制限がかかったアカウントで表示されます。利用したいプラットフォームでの Minecraft プレイを、保護者アカウントから子どものアカウントに許可する必要があります。account.microsoft.com/family でファミリー設定を調整してください。"
  - question: "Xbox Live 認証エラーが何度も出ます。どうすればいいですか？"
    answer: "Xbox Live エラーは通常、Microsoft アカウントの国/地域設定が Xbox Live を許可していない、または Xbox Live の利用規約に同意していないことが原因です。同じ Microsoft アカウントで一度 xbox.com にサインインして規約に同意し、その後 GDLauncher で再度試してください。"
  - question: "GDLauncher を使うために Minecraft を買い直す必要がありますか？"
    answer: "いいえ。GDLauncher は既存の Microsoft / Mojang の Minecraft アカウントを使用します。別途購入やサブスクリプションは不要です。すでに Minecraft Java Edition を所有していれば、同じアカウントで GDLauncher にサインインできます。"
---

# Microsoft 認証エラー

GDLauncher で Microsoft アカウントにサインインするとき、ランチャーはあなたの代わりに Microsoft の OAuth サービスと Mojang の認証 API と通信します。これらのサービスから返されるエラーはそのままランチャーに表示され、文言は GDLauncher ではなく Microsoft が決めています。

以下は最もよく出るものとその意味です。

## Invalid Grant

Microsoft が OAuth 交換を拒否したときに表示されます。よくある原因:

- アカウントにパスワードが設定されていない (メールリンクやソーシャルログインで作成された Microsoft アカウント)。[account.microsoft.com](https://account.microsoft.com) でパスワードを追加してください。
- アカウントが 2 段階認証なしの古いサインインフローを使用している。[account.microsoft.com/security](https://account.microsoft.com/security) で 2FA を有効化すると、ほとんどの場合これで解決します。
- キャッシュされたトークンが古くなっている。**Settings → Accounts** でアカウントをサインアウトし、再度サインインしてください。

## アカウント BAN

GDLauncher は Mojang の応答をそのまま中継します。BAN は Mojang 側で行われ、GDLauncher は独自の BAN リストを持っていません。BAN の理由や異議申し立て方法は、同じアカウントで [minecraft.net](https://minecraft.net) にサインインして確認してください。

## コンソールアクセスが必要

これは通常、Microsoft ファミリーグループ内の子どもアカウントで表示されます。保護者アカウントが [account.microsoft.com/family](https://account.microsoft.com/family) で子どもに Minecraft Java Edition を許可する必要があります。許可後、GDLauncher 内でいったんサインアウトして再度サインインしてください。

## Xbox Live エラー

Xbox Live の失敗はほとんどの場合 2 つのいずれかです:

- Microsoft アカウントの国/地域設定が Xbox Live を許可していない。[account.microsoft.com/profile](https://account.microsoft.com/profile) で調整してください。
- アカウントが Xbox Live の利用規約に同意していない。同じ Microsoft アカウントで一度 [xbox.com](https://xbox.com) にサインインして規約に同意し、その後 GDLauncher で再度試してください。

## アカウントの期限切れ

Microsoft のリフレッシュトークンが期限切れになったか取り消されました (多くの場合、別の場所でアカウントのパスワードを変更したためです)。GDLauncher は「Account expired」プロンプトを表示し、再認証を提案します。**Settings → Accounts** から再度サインインしてください。

## どうしても解決しないとき

エラーメッセージが上記のどれにも当てはまらない場合、アプリレベルのログを両方 [Discord](https://discord.gdlauncher.com) で共有してください: `main.log` (Electron) と最新の `__gdl_logs__/<timestamp>.log` (Rust コア)。場所については [Share App Logs](/guides/share-app-logs) を参照。認証フローは 2 つのプロセス間にまたがるため、両方が必要なケースがほとんどです。
