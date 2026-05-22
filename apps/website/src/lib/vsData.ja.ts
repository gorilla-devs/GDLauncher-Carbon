import type { LocaleData } from "./vsData"

const ja: LocaleData = {
  chrome: {
    compareBreadcrumb: "比較",
    feature: "機能",
    tryGdl: "GDLauncher を試す",
    seeAllComparisons: "すべての比較を見る",
    theVerdict: "結論",
  },
  hub: {
    pageTitle: "GDLauncher と他の Minecraft ランチャーの比較",
    pageDescription:
      "GDLauncher と他の人気 Minecraft ランチャーの詳細比較: Prism Launcher、CurseForge App、Modrinth App、ATLauncher、MultiMC、FTB App、TLauncher。",
    h1: "GDLauncher の比較",
    intro:
      "どの Minecraft ランチャーを選ぶか迷っていますか? GDLauncher が主要な代替ランチャーとどう違うのか、機能ごとに比較しました。私たちは当事者なので公平ではありませんが、書面で比較しているので、ご自身で判断していただけます。",
    competitors: {
      prismlauncher: {
        blurb:
          "軽量・オープンソースな MultiMC のフォーク。GDLauncher との使い勝手と modpack 対応の比較。",
      },
      "curseforge-app": {
        blurb:
          "CurseForge の公式ランチャー。CurseForge 連携、Modrinth サポート、内蔵サーバー管理を比較します。",
      },
      "modrinth-app": {
        blurb:
          "Modrinth 専用ランチャー。GDLauncher なら Modrinth と CurseForge の両方を 1 箇所で使えます。",
      },
      atlauncher: {
        blurb:
          "ベテランの modpack ランチャー。UI、パフォーマンス、プラットフォーム対応を並べて比較。",
      },
      multimc: {
        blurb:
          "軽量でパワーユーザー向けのランチャー。自動化と modpack のワークフローでの違い。",
      },
      "ftb-app": {
        blurb:
          "Feed The Beast 公式の FTB / CurseForge パック向けランチャー。Modrinth 対応、Cloud Instance Sharing、サーバー管理の違い。",
      },
      tlauncher: {
        blurb:
          "Mojang 認証をスキップするランチャー。EULA に違反する点と、それを使うことで失うもの。",
      },
    },
  },
  comparisons: {
    prismlauncher: {
      title: "GDLauncher と Prism Launcher",
      description:
        "GDLauncher と Prism Launcher の詳細比較: 機能、modpack 対応、パフォーマンス、UI。あなたに合う Minecraft ランチャーを見つけましょう。",
      intro:
        "Prism Launcher は人気のオープンソース MultiMC フォークです。GDLauncher は CurseForge と Modrinth の深い統合を持つモダンなランチャーです。日々使う上で本当に重要なポイントで両者を比較します。",
      rows: [
        {
          feature: "CurseForge 対応",
          gdl: "あり",
          competitor: "一部対応 (回避策必要)",
          note: "mod 作者がサードパーティ API 経由のダウンロードを許可していない場合、Prism は該当ファイルをブラウザで手動ダウンロードするよう求めます",
        },
        {
          feature: "Modrinth 対応",
          gdl: "あり",
          competitor: "あり",
        },
        { feature: "Java 自動管理", gdl: "あり", competitor: "あり" },
        { feature: "Mod の自動更新", gdl: "あり", competitor: "なし (手動チェックのみ)" },
        { feature: "Modpack の自動更新", gdl: "あり", competitor: "なし (手動チェックのみ)" },
        { feature: "マルチインスタンス", gdl: "あり", competitor: "あり" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "あり (ワンクリックコード, CF + MR ミックス対応)",
          competitor: "なし (手動エクスポート, CF + MR ミックス非対応)",
        },
        { feature: "サーバー管理", gdl: "あり (内蔵)", competitor: "なし" },
        { feature: "モダンな UI", gdl: "あり", competitor: "なし" },
        {
          feature: "アドオン作者への報酬",
          gdl: "あり",
          competitor: "なし",
        },
        { feature: "GitHub でソース公開", gdl: "あり", competitor: "あり" },
        { feature: "軽量 (RAM)", gdl: "なし", competitor: "あり" },
      ],
      verdict:
        "Prism は、シンプルで軽量なランチャーが好きで、modpack 周りの手間を厭わないなら最適です。一方 GDLauncher は、CurseForge と Modrinth からのワンクリックインストール、Cloud Instance Sharing、内蔵サーバー管理をアプリから離れずに済ませたい方向けです。Mod 入り Minecraft が初めての方や、ミニマルさより仕上がりを重視する方には GDLauncher の方が簡単です。",
      sections: [
        {
          heading: "Modpack のワークフロー",
          paragraphs: [
            "Prism も GDLauncher も、ランチャー内から CurseForge パックを直接ブラウズしてインストールできます。普段使いの体験はほぼ同じです。違いが出るのは端のケースで、mod 作者がサードパーティ API 経由のダウンロードを許可していない場合、Prism はそれらの該当ファイルを毎回ブラウザで手動ダウンロードするよう求めてきます。GDLauncher は CurseForge とのパートナーシップでそのファイルも直接取得するので、ブロックされた mod を含むパックでもワンクリックのまま進みます。",
            "Modrinth パックは両ランチャーとも同じで、アプリ内のブラウザからワンクリックでインストールできます。",
          ],
        },
        {
          heading: "UI と発見性",
          paragraphs: [
            "Prism の Qt ベースの UI は機能的ですが実用一辺倒で、メイン画面はインスタンスのリストです。GDLauncher の UI は modpack の発見と管理に特化して設計されており、内蔵ブラウザ、インスタンスのグループ化、ドラッグ&ドロップでの並び替え、ビジュアルカードを備えています。主観ですが、スクリーンショットを見比べる価値があります。",
          ],
        },
        {
          heading: "Cloud Instance Sharing",
          paragraphs: [
            "GDLauncher にはワンクリックの Cloud Instance Sharing があります。コードを貼り付けるだけで同じセットアップが手に入ります。Prism はファイル経由のインスタンスエクスポート/インポートで、機能はしますが友人との共有はやや手間がかかります。",
          ],
        },
      ],
    },
    "curseforge-app": {
      title: "GDLauncher と CurseForge App",
      description:
        "GDLauncher と CurseForge App の比較: 機能、広告、Modrinth 対応、サーバー管理。Mod 入り Minecraft を遊ぶならどちらか。",
      intro:
        "CurseForge App は CurseForge コンテンツ向けの公式ランチャーです。GDLauncher も CurseForge と統合され、加えて同じブラウザで Modrinth、両プラットフォームをまたぐCloud Instance Sharing、そして内蔵サーバー管理を備えます。違いを見ていきましょう。",
      rows: [
        {
          feature: "CurseForge 対応",
          gdl: "あり",
          competitor: "あり (ネイティブ、自社アプリ)",
        },
        { feature: "Modrinth 対応", gdl: "あり", competitor: "なし" },
        { feature: "Java 自動管理", gdl: "あり", competitor: "あり" },
        { feature: "Mod の自動更新", gdl: "あり", competitor: "あり (確認あり)" },
        { feature: "Modpack の自動更新", gdl: "あり", competitor: "あり (確認あり)" },
        { feature: "マルチインスタンス", gdl: "あり", competitor: "あり" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "あり (ワンクリックコード, CF + MR ミックス対応)",
          competitor: "あり (CurseForge のみ)",
        },
        { feature: "サーバー管理", gdl: "あり (内蔵)", competitor: "なし" },
        {
          feature: "アプリ内広告",
          gdl: "あり (アプリ内バナー)",
          competitor: "あり (アプリ内バナー)",
        },
        { feature: "GitHub でソース公開", gdl: "あり", competitor: "なし" },
        { feature: "アドオン作者への報酬", gdl: "あり", competitor: "あり" },
      ],
      verdict:
        "CurseForge コンテンツしか入れないなら、CurseForge App が公式の選択です。GDLauncher は同じ CurseForge 連携に加え、同じブラウザで Modrinth、CurseForge と Modrinth が混在するセットアップごと持ち運べるCloud Instance Sharing、そして内蔵サーバー管理を備えます。",
      sections: [
        {
          heading: "1 つのランチャー内に Modrinth",
          paragraphs: [
            "CurseForge App は設計上 CurseForge 専用です。Modrinth は Fabric mod、軽量化 mod、shader を中心に急成長しており、両プラットフォームに公開する作者も増えています。GDLauncher の内蔵ブラウザは両方を同時に検索するので、どちらを選ぶか悩まずに済みます。",
          ],
        },
        {
          heading: "サーバー管理",
          paragraphs: [
            "GDLauncher には Minecraft サーバーの管理機能が内蔵されています。Vanilla、Forge、Fabric、NeoForge、Quilt サーバーを作成し、シングルプレイのインスタンスと同じ UI で管理できます。CurseForge App にはサーバー管理機能はありません。",
          ],
        },
        {
          heading: "Cloud Instance Sharing",
          paragraphs: [
            "どちらのランチャーも友達とセットアップを共有できます。CurseForge App は CurseForge エコシステム内で完結します。CurseForge の modpack なら渡せますが、CurseForge mod と Modrinth mod が混ざったセットアップはそのまま渡せません。GDLauncher のCloud Instance Sharingはその混在ケースにも対応します。コードを 1 つ貼り付けるだけで、相手側に両プラットフォームのファイルを元の CDN から再ダウンロードして、あなたの状態そのままのインスタンスが手元に揃います。",
          ],
        },
      ],
    },
    "modrinth-app": {
      title: "GDLauncher と Modrinth App",
      description:
        "GDLauncher と Modrinth App の比較: mod と modpack に最適なランチャーは? 機能、プラットフォーム、エコシステムの対応を比較。",
      intro:
        "Modrinth App は公式の Modrinth ランチャーで、Modrinth のコンテンツしか使わないなら良い選択肢です。GDLauncher も Modrinth と統合され、加えて CurseForge、Cloud Instance Sharing、サーバー管理を備えます。並べて見てみましょう。",
      rows: [
        {
          feature: "CurseForge 対応",
          gdl: "あり",
          competitor: "なし",
        },
        {
          feature: "Modrinth 対応",
          gdl: "あり",
          competitor: "あり (ネイティブ、自社アプリ)",
        },
        { feature: "Java 自動管理", gdl: "あり", competitor: "あり" },
        { feature: "Mod の自動更新", gdl: "あり", competitor: "あり (確認あり)" },
        { feature: "Modpack の自動更新", gdl: "あり", competitor: "あり (確認あり)" },
        { feature: "マルチインスタンス", gdl: "あり", competitor: "あり" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "あり (ワンクリックコード, CF + MR ミックス対応)",
          competitor: "なし (手動エクスポート, Modrinth のみ)",
        },
        { feature: "サーバー管理", gdl: "あり (内蔵)", competitor: "あり (Modrinth Hosting)" },
        { feature: "モダンな UI", gdl: "あり", competitor: "あり" },
        { feature: "GitHub でソース公開", gdl: "あり", competitor: "あり" },
        { feature: "アドオン作者への報酬", gdl: "あり", competitor: "あり" },
        { feature: "軽量", gdl: "中程度", competitor: "中程度" },
      ],
      verdict:
        "Modrinth App は、完全に Modrinth エコシステムだけで暮らすなら素晴らしい選択肢です。しかし最も人気のある modpack の多く (RLCraft、ATM10、DawnCraft、FTB シリーズ) は今でも CurseForge 専用で、両プラットフォームに公開されているパックでも通常は CurseForge が先行します。GDLauncher なら Modrinth と CurseForge が 1 つのブラウザに収まり、Cloud Instance Sharing と内蔵サーバー管理まで付きます。エコシステムの幅を取るなら GDLauncher、Modrinth 専用でフォーカスしたい場合は Modrinth App がおすすめです。",
      sections: [
        {
          heading: "CurseForge のギャップ",
          paragraphs: [
            "最も大きな違いはシンプルです: Modrinth App は CurseForge コンテンツをインストールできません。Modrinth のみの mod ならこれは問題になりません。しかし CurseForge には依然として大きな modpack ライブラリと、CurseForge 限定の古い Forge mod が多数あります。GDLauncher のブラウザは両プラットフォームを 1 度の検索で表示するので、必要なバージョンがある方を選べます。",
          ],
        },
        {
          heading: "両方のエコシステムが素晴らしい",
          paragraphs: [
            "Modrinth はライブラリが小さい代わりに、軽快で広告なしのサイトと、モッダー向けの優れた API を持っています。CurseForge はカタログが豊富で、歴史あるパックも揃っています。人気の mod は今やほとんど両方に公開されています。GDLauncher の戦略は、ユーザーに選ばせるのではなく、両方をネイティブにサポートすることです。",
          ],
        },
        {
          heading: "サーバー管理と Cloud Instance Sharing",
          paragraphs: [
            "Modrinth のサーバー管理は有料の Modrinth Hosting 連携です。Modrinth でサーバーをプロビジョニングし、アプリ内で管理します。GDLauncher のサーバー管理はローカルで、自分のマシンに Vanilla / Forge / Fabric / NeoForge / Quilt サーバーを立て、ライブコンソールやプレイヤー管理、シングルプレイと同じインスタンス設定をそのまま使えます。ホスティング料金は不要です。",
            "Cloud Instance Sharing は Modrinth App にはないもう一つの GDLauncher の機能です。コードを貼り付けるだけで CurseForge と Modrinth を混在させたまま同じセットアップが手に入ります。",
          ],
        },
      ],
    },
    atlauncher: {
      title: "GDLauncher と ATLauncher",
      description:
        "GDLauncher と ATLauncher の詳細比較: UI、modpack 対応、サーバー管理、開発者体験。どちらが優れた Minecraft ランチャーか?",
      intro:
        "ATLauncher は長年運営されている Java ベースの modpack ランチャーで、独自の ATLauncher パックエコシステムを持っています。GDLauncher はより新しい Rust + Solid の代替で、モダンな UI と CurseForge / Modrinth のワンクリックインストールを備えています。両者を比較します。",
      rows: [
        {
          feature: "CurseForge 対応",
          gdl: "あり",
          competitor: "一部対応 (回避策必要)",
          note: "mod 作者がサードパーティ API 経由のダウンロードを許可していない場合、ATLauncher は該当ファイルをブラウザで手動ダウンロードするよう求めます",
        },
        { feature: "Modrinth 対応", gdl: "あり", competitor: "あり" },
        { feature: "Java 自動管理", gdl: "あり", competitor: "あり" },
        { feature: "Mod の自動更新", gdl: "あり", competitor: "あり (確認あり)" },
        { feature: "Modpack の自動更新", gdl: "あり", competitor: "あり (確認あり)" },
        { feature: "マルチインスタンス", gdl: "あり", competitor: "あり" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "あり (ワンクリックコード, CF + MR ミックス対応)",
          competitor: "なし (手動エクスポート, CF + MR ミックス非対応)",
        },
        { feature: "サーバー管理", gdl: "あり (内蔵)", competitor: "なし" },
        {
          feature: "モダンな UI",
          gdl: "あり",
          competitor: "一部 (FlatLaf 適用の Java Swing)",
        },
        { feature: "アドオン作者への報酬", gdl: "あり", competitor: "なし" },
        { feature: "GitHub でソース公開", gdl: "あり", competitor: "あり" },
        {
          feature: "独自 modpack の公開",
          gdl: "あり (Cloud Instance Sharing のワンクリックコード)",
          competitor: "あり (ATLauncher パック)",
        },
      ],
      verdict:
        "ATLauncher は、ATLauncher のキュレーション済みパックリストが目当てだったり、既にそのワークフローに慣れているなら手堅い選択肢です。GDLauncher の強みは、モダンな UI、より深い CurseForge 連携、Cloud Instance Sharing、内蔵サーバー管理です。2026 年の Mod 入り Minecraft プレイヤーの大半にとって、GDLauncher の体験はモダンなアプリに期待するものに近いと思います。",
      sections: [
        {
          heading: "UI 世代の差",
          paragraphs: [
            "ATLauncher は Java Swing に最新の FlatLaf ルックアンドフィールを重ねて使っています。クラシックな Swing と比べれば確かな進歩ですが、密度、モーション、プラットフォーム感の面ではネイティブなモダンランチャーには及びません。GDLauncher は Solid で構築され、UnoCSS ベースの独自デザインシステム、ネイティブ感覚のドラッグ&ドロップ、アニメーション、グループ化を備えています。",
          ],
        },
        {
          heading: "CurseForge 連携",
          paragraphs: [
            "ATLauncher と GDLauncher はどちらもランチャー内で CurseForge パックを閲覧・インストールできるので、日常の体験は似ています。差は端の部分で出ます。mod 作者がサードパーティ API でのダウンロードを無効化している場合、ATLauncher はその都度ブロックされたリンクをクリックし、ファイルをブラウザで手動ダウンロードするよう求めます。GDLauncher の CurseForge パートナーシップはそうしたファイルも直接フェッチするので、ブロック対象 mod を含むパックでもワンクリックインストールのままです。",
          ],
        },
        {
          heading: "ATLauncher パック と Cloud Instance Sharing",
          paragraphs: [
            "ATLauncher は独自のパックエコシステムを運営しています。GDLauncher はそこでは戦わず、代わりに Cloud Instance Sharing によって誰でも自分のセットアップ (mod、設定、設定値) をワンクリックコードで共有できます。哲学が違うだけなので、自分や友人の遊び方に合うものを選んでください。",
          ],
        },
      ],
    },
    multimc: {
      title: "GDLauncher と MultiMC",
      description:
        "GDLauncher と MultiMC の詳細比較: 機能、自動化、modpack の扱い、モダンな UI。あなたに合う Minecraft ランチャーを見つけましょう。",
      intro:
        "MultiMC はマルチインスタンス型の Minecraft 起動を先駆けたランチャーですが、最後の公式リリースは 2021 年 12 月の 0.6.14 で、活発な開発の大半はフォーク (とりわけ Prism Launcher) に移っています。GDLauncher はモダンで、強い自動化を持つランチャーです。現実的な比較を見ていきましょう。",
      rows: [
        {
          feature: "CurseForge 対応",
          gdl: "あり",
          competitor: "なし",
        },
        { feature: "Modrinth 対応", gdl: "あり", competitor: "あり" },
        { feature: "Java 自動管理", gdl: "あり", competitor: "なし" },
        { feature: "Mod の自動更新", gdl: "あり", competitor: "なし" },
        { feature: "Modpack の自動更新", gdl: "あり", competitor: "なし" },
        {
          feature: "マルチインスタンス",
          gdl: "あり",
          competitor: "あり (得意分野)",
        },
        {
          feature: "Cloud Instance Sharing",
          gdl: "あり (ワンクリックコード, CF + MR ミックス対応)",
          competitor: "なし (手動エクスポート, CF + MR ミックス非対応)",
        },
        { feature: "サーバー管理", gdl: "あり (内蔵)", competitor: "なし" },
        { feature: "モダンな UI", gdl: "あり", competitor: "なし" },
        { feature: "アドオン作者への報酬", gdl: "あり", competitor: "なし" },
        { feature: "GitHub でソース公開", gdl: "あり", competitor: "あり" },
        { feature: "軽量", gdl: "なし", competitor: "あり (とても軽い)" },
      ],
      verdict:
        "MultiMC は、コンパクトで非常に柔軟性の高いランチャーが欲しく、Java の設定や mod の管理、更新を自分で行うのが苦にならないなら良い選択肢です。GDLauncher は、Java の自動管理、自動更新、ワンクリックインストール、Cloud Instance Sharing、サーバー管理を自動で済ませたいプレイヤー向けで、MultiMC が先駆けたマルチインスタンスのワークフローはそのまま受け継いでいます。",
      sections: [
        {
          heading: "自動化と制御",
          paragraphs: [
            "MultiMC の設計思想は「ユーザーが頼んでいないことはしない」です。つまり Java のパスもバージョンも、mod の管理も更新も全部自分で行います。パワーユーザーはこれを好みますが、新規プレイヤーは離れていきます。",
            "GDLauncher は逆のアプローチを取ります。インスタンスごとに何が必要かを検出し、インストールして、最新に保ちます。一方で、オーバーライドしたい場合に備えて同じ調整項目をすべてインスタンス設定に用意してあります。デフォルトでも動くし、必要な制御も揃っています。",
          ],
        },
        {
          heading: "Modpack の扱い",
          paragraphs: [
            "MultiMC には Modrinth ブラウザは内蔵されていますが、CurseForge との統合はありません。CurseForge の pack をプレイするには、zip ファイルを手動でインポートするか、サードパーティのツールでマニフェストを取得する必要があります。GDLauncher のブラウザは CurseForge と Modrinth を並べて表示し、両方ともワンクリックでインストールできます。",
          ],
        },
        {
          heading: "系譜",
          paragraphs: [
            "MultiMC は 2021 年 12 月以降、新しいリリースを出していません。プロジェクトのエネルギーは事実上 Prism Launcher などのフォークへと移っています。長年 MultiMC を使ってきて、ワークフローを失わずによりモダンな UI が欲しいなら Prism が自然な乗り換え先です。GDLauncher はもっと大きな飛躍 (自動化が多く、手動ステップが少ない) になります。両方試して、自分の使い方に合うモデルを選んでください。",
          ],
        },
        {
          heading: "Cloud Instance Sharing",
          paragraphs: [
            "MultiMC で友達とセットアップを共有するということは、インスタンスを zip にエクスポートしてファイルを渡すことです。それでも動きますが、ファイルをどこかにホストする必要があり、相手も同じ手順でインポートする必要があります。GDLauncher のCloud Instance Sharingはそれを短いコードに置き換えます。コードを貼り付ければ、ランチャーが GDL サービスからスナップショットを取得し、mod は元の CDN から再ダウンロードされます。コード 1 つで、CurseForge + Modrinth の混在コンテンツを同じ共有に含められ、zip ファイルの受け渡しは不要です。",
          ],
        },
      ],
    },
  },
}

export default ja
