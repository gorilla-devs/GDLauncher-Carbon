/**
 * Guides hub data, translated guide titles, descriptions, and category
 * labels per locale. Brand names (CurseForge, Modrinth, Forge, Fabric, etc.)
 * are kept as-is across translations.
 *
 * Slugs are stable English; the href for a given locale is built with
 * `localizedPath()` at the call site so the default locale stays at the
 * root and others get the `/<locale>/` prefix.
 */
import type { Locale } from "./i18n"
import { localizedPath } from "./i18n"

export interface GuideEntry {
  slug: string
  title: string
  description: string
  category: string
  /**
   * Optional href override. When set, the hub uses this exact path instead
   * of `localizedPath("/guides/" + slug, locale)`. Used by the launcher-
   * comparison entries, which live under `/vs/<slug>` outside the /guides/
   * tree.
   */
  href?: string
}

export interface GuidesHubChrome {
  pageTitle: string
  pageDescription: string
  breadcrumb: string
  h1Main: string
  h1Highlight: string
  intro: string
}

interface CategoryLabels {
  installation: string
  modloaders: string
  instanceManagement: string
  sharing: string
  performance: string
  servers: string
  system: string
  platforms: string
  launcherComparisons: string
}

interface GuideMeta { title: string; description: string }

interface LocaleData {
  chrome: GuidesHubChrome
  categories: CategoryLabels
  guides: Record<string, GuideMeta>
  /**
   * Launcher comparison entries keyed by /vs/<slug>. Same shape as `guides`
   * but rendered under the `launcherComparisons` category with an href that
   * points to /vs/<slug>.
   */
  comparisons: Record<string, GuideMeta>
}

const SLUG_ORDER: Array<[string, keyof CategoryLabels]> = [
  ["install-modpack", "installation"],
  ["install-mods", "installation"],
  ["install-shaders", "installation"],
  ["install-resourcepacks", "installation"],
  ["install-datapacks", "installation"],
  ["install-worlds", "installation"],
  ["install-forge", "modloaders"],
  ["install-neoforge", "modloaders"],
  ["install-fabric", "modloaders"],
  ["install-quilt", "modloaders"],
  ["switch-mod-loader", "modloaders"],
  ["update-modpack-version", "instanceManagement"],
  ["update-mods", "instanceManagement"],
  ["change-minecraft-version", "instanceManagement"],
  ["change-instance-icon", "instanceManagement"],
  ["duplicate-an-instance", "instanceManagement"],
  ["delete-instances-safely", "instanceManagement"],
  ["group-instances-and-favorites", "instanceManagement"],
  ["open-instance-files", "instanceManagement"],
  ["find-logs-and-crash-reports", "instanceManagement"],
  ["import-an-instance", "sharing"],
  ["export-an-instance", "sharing"],
  ["share-an-instance", "sharing"],
  ["allocate-more-ram", "performance"],
  ["performance-mods", "performance"],
  ["enable-potato-mode", "performance"],
  ["allocate-server-ram", "servers"],
  ["setup-minecraft-server", "servers"],
  ["move-runtime-path", "system"],
  ["backup-gdlauncher-data", "system"],
  ["share-app-logs", "system"],
  ["change-app-theme", "system"],
  ["manage-microsoft-accounts", "system"],
  ["manage-java-versions", "system"],
  ["curseforge-vs-modrinth", "platforms"],
]

// Launcher comparison entries. These point to /vs/<slug> instead of
// /guides/<slug>, and the destination pages are English-only (not in
// LOCALIZED_PATH_PREFIXES). Titles/descriptions get translated for
// discovery in the hub even though the destination is English.
const COMPARISON_ORDER: string[] = [
  "prismlauncher",
  "curseforge-app",
  "modrinth-app",
  "atlauncher",
  "multimc",
  "ftb-app",
  "tlauncher",
]

const data: Record<Locale, LocaleData> = {
  en: {
    chrome: {
      pageTitle: "Minecraft Modding Guides | GDLauncher",
      pageDescription: "Learn how to install modpacks, mods, shaders, and more for Minecraft. Step-by-step tutorials and guides for modded Minecraft.",
      breadcrumb: "Guides",
      h1Main: "Minecraft Modding",
      h1Highlight: "Guides",
      intro: "Learn how to install mods, modpacks, shaders, and more. Step-by-step tutorials to get you started with modded Minecraft.",
    },
    categories: {
      installation: "Installation Guides",
      modloaders: "Mod Loaders",
      instanceManagement: "Instance Management",
      sharing: "Sharing & Migration",
      performance: "Performance",
      servers: "Servers",
      system: "System & Accounts",
      platforms: "Platform Guides",
      launcherComparisons: "Launcher Comparisons",
    },
    guides: {
      "install-modpack": { title: "How to Install Modpacks", description: "Learn how to install Minecraft modpacks with GDLauncher in just a few clicks." },
      "install-mods": { title: "How to Install Mods", description: "Learn how to find and install individual Minecraft mods from CurseForge and Modrinth." },
      "install-shaders": { title: "How to Install Shaders", description: "Transform your Minecraft graphics with beautiful shader packs." },
      "install-resourcepacks": { title: "How to Install Resource Packs", description: "Customize Minecraft's textures and sounds with resource packs." },
      "install-datapacks": { title: "How to Install Data Packs", description: "Add new recipes, loot tables, and mechanics without mods." },
      "install-worlds": { title: "How to Install Worlds", description: "Play adventure maps and custom world downloads in Minecraft." },
      "install-forge": { title: "How to Install Forge", description: "Step-by-step guide to installing Minecraft Forge with GDLauncher." },
      "install-neoforge": { title: "How to Install NeoForge", description: "Set up the modern Forge fork where most big content mods now publish first." },
      "install-fabric": { title: "How to Install Fabric", description: "Learn how to set up Fabric mod loader for Minecraft using GDLauncher." },
      "install-quilt": { title: "How to Install Quilt", description: "Install the Fabric-superset loader that runs most Fabric mods plus its own." },
      "switch-mod-loader": { title: "Switch Mod Loaders on an Existing Instance", description: "Change Forge to NeoForge, Fabric to Quilt, or any other combination without recreating the instance." },
      "update-modpack-version": { title: "Update a Modpack to a New Version", description: "Move a CurseForge or Modrinth modpack instance to a newer release without losing worlds or settings." },
      "update-mods": { title: "Update Mods in an Instance", description: "Keep your installed Minecraft mods current with per-mod and Update All actions." },
      "change-minecraft-version": { title: "Change the Minecraft Version of an Instance", description: "Switch a custom instance to a different Minecraft version while keeping the instance folder intact." },
      "change-instance-icon": { title: "Change an Instance's Icon", description: "Personalize each Minecraft instance with a custom PNG or JPG icon from your computer." },
      "duplicate-an-instance": { title: "Duplicate an Instance", description: "Make a full copy of an existing instance, useful for testing mod changes safely or branching a playthrough." },
      "delete-instances-safely": { title: "Delete Instances Safely", description: "Remove single or multiple Minecraft instances, what gets kept, and how to recover from accidental deletion." },
      "group-instances-and-favorites": { title: "Group Instances and Mark Favorites", description: "Organize a large library with named groups and a favorites bar." },
      "open-instance-files": { title: "Open Instance Files", description: "Jump to the instance folder in your OS file manager to back up worlds, edit configs, or grab screenshots." },
      "find-logs-and-crash-reports": { title: "Find Logs and Crash Reports", description: "Where GDLauncher stores logs and crash reports, how to view them in-app, and how to attach them to a support request." },
      "import-an-instance": { title: "Import an Instance", description: "Import an existing Minecraft instance from a CurseForge .zip, Modrinth .mrpack, GDL share code, or another launcher." },
      "export-an-instance": { title: "Export an Instance", description: "Export a GDLauncher instance as a CurseForge-format .zip you can share, archive, or upload." },
      "share-an-instance": { title: "Share an Instance with a Friend", description: "Generate a GDLauncher share code so a friend can install the same mods, configs, and resource packs you have." },
      "allocate-more-ram": { title: "Allocate More RAM to Minecraft", description: "Improve performance by allocating more memory to Minecraft." },
      "performance-mods": { title: "Performance Mods Guide", description: "Boost your FPS with these essential Minecraft performance mods." },
      "enable-potato-mode": { title: "Enable Potato PC Mode", description: "Turn off launcher animations to reduce CPU and GPU load on older machines." },
      "allocate-server-ram": { title: "Allocate More RAM to a Minecraft Server", description: "Increase the memory available to a Minecraft server for more players, mods, and chunk load." },
      "setup-minecraft-server": { title: "Set Up a Minecraft Server", description: "Create and run a Minecraft server from GDLauncher with EULA acceptance, live console, and metrics." },
      "move-runtime-path": { title: "Move GDLauncher's Data to Another Drive", description: "Change the runtime path to move all instances and assets to a different drive, with migration handled automatically." },
      "backup-gdlauncher-data": { title: "Back Up GDLauncher Before Formatting Your PC", description: "Save every instance, account, setting, and world before a reformat or PC change, then restore everything without losing progress." },
      "share-app-logs": { title: "Share GDLauncher App Logs", description: "Find your launcher's app-wide logs on Windows, macOS, and Linux, then attach them to a Discord or GitHub support request." },
      "change-app-theme": { title: "Change GDLauncher's Theme", description: "Pick a different look for the launcher with built-in themes and live preview." },
      "manage-microsoft-accounts": { title: "Add, Switch, and Remove Microsoft Accounts", description: "Sign in to multiple accounts, choose which one Play uses, refresh expired tokens, and remove accounts." },
      "manage-java-versions": { title: "Manage Java Versions and Per-Instance Override", description: "Add custom Java installations, create profiles, and set a per-instance Java override." },
      "curseforge-vs-modrinth": { title: "CurseForge vs Modrinth", description: "Compare the two major Minecraft mod platforms and their differences." },
    },
    comparisons: {
      "prismlauncher": { title: "GDLauncher vs Prism Launcher", description: "How the two stack up on CurseForge support, automation, Cloud Instance Sharing, and modern UI." },
      "curseforge-app": { title: "GDLauncher vs CurseForge App", description: "Modrinth and CurseForge in one launcher, plus Cloud Instance Sharing and built-in server management." },
      "modrinth-app": { title: "GDLauncher vs Modrinth App", description: "Both Modrinth and CurseForge in one launcher, plus Cloud Instance Sharing and server management." },
      "atlauncher": { title: "GDLauncher vs ATLauncher", description: "Modern UI and deeper CurseForge integration compared with the long-running Java launcher." },
      "multimc": { title: "GDLauncher vs MultiMC", description: "Automation, modpack workflows, and modern UX compared with MultiMC's minimalist roots." },
      "ftb-app": { title: "GDLauncher vs FTB App", description: "Modrinth, Cloud Instance Sharing, and server management compared with the Feed The Beast launcher." },
      "tlauncher": { title: "GDLauncher vs TLauncher", description: "Official Microsoft sign-in vs an EULA-violating authentication bypass, and what you give up using the latter." },
    },
  },
  ja: {
    chrome: {
      pageTitle: "Minecraft Mod ガイド | GDLauncher",
      pageDescription: "Minecraft のモッドパック、Mod、シェーダーなどのインストール方法を解説。Mod 環境を始めるためのステップバイステップ チュートリアル。",
      breadcrumb: "ガイド",
      h1Main: "Minecraft Mod",
      h1Highlight: "ガイド",
      intro: "Mod、モッドパック、シェーダーなどのインストール方法を学べます。Mod 入りの Minecraft を始めるためのステップバイステップ チュートリアル。",
    },
    categories: {
      installation: "インストール ガイド",
      modloaders: "Mod ローダー",
      instanceManagement: "インスタンスの管理",
      sharing: "共有と移行",
      performance: "パフォーマンス",
      servers: "サーバー",
      system: "システムとアカウント",
      platforms: "プラットフォーム ガイド",
      launcherComparisons: "他ランチャーとの比較",
    },
    guides: {
      "install-modpack": { title: "モッドパックのインストール方法", description: "GDLauncher で Minecraft のモッドパックを数クリックでインストールする方法。" },
      "install-mods": { title: "Mod のインストール方法", description: "CurseForge と Modrinth から個別の Minecraft Mod を見つけてインストールする方法。" },
      "install-shaders": { title: "シェーダーのインストール方法", description: "美しいシェーダーパックで Minecraft のグラフィックを一新。" },
      "install-resourcepacks": { title: "リソースパックのインストール方法", description: "リソースパックで Minecraft のテクスチャとサウンドをカスタマイズ。" },
      "install-datapacks": { title: "データパックのインストール方法", description: "Mod を使わずに新しいレシピ、ルートテーブル、ゲーム挙動を追加。" },
      "install-worlds": { title: "ワールドのインストール方法", description: "アドベンチャーマップやカスタムワールドを Minecraft で遊ぶ方法。" },
      "install-forge": { title: "Forge のインストール方法", description: "GDLauncher で Minecraft Forge を導入する手順を解説。" },
      "install-neoforge": { title: "NeoForge のインストール方法", description: "大型コンテンツ Mod の主軸となっているモダンな Forge フォークをセットアップ。" },
      "install-fabric": { title: "Fabric のインストール方法", description: "GDLauncher で Fabric Mod ローダーをセットアップする方法。" },
      "install-quilt": { title: "Quilt のインストール方法", description: "ほとんどの Fabric Mod に加えて独自 Mod も動く Fabric 上位互換ローダー。" },
      "switch-mod-loader": { title: "既存インスタンスの Mod ローダーを切替", description: "Forge から NeoForge、Fabric から Quilt など、インスタンスを作り直さずに変更。" },
      "update-modpack-version": { title: "Modpack を新バージョンに更新", description: "CurseForge / Modrinth の Modpack インスタンスを、ワールドや設定を保ったまま更新。" },
      "update-mods": { title: "インスタンスの Mod を更新", description: "個別の Mod を 1 クリック、または Update All で一括更新。" },
      "change-minecraft-version": { title: "インスタンスの Minecraft バージョンを変更", description: "インスタンスフォルダーを保ったまま MC バージョンを切り替え。" },
      "change-instance-icon": { title: "インスタンスのアイコンを変更", description: "PNG / JPG 画像で各インスタンスを個別にカスタマイズ。" },
      "duplicate-an-instance": { title: "インスタンスを複製", description: "既存インスタンスを丸ごとコピー。Mod 変更の安全テストやプレイ分岐に。" },
      "delete-instances-safely": { title: "インスタンスを安全に削除", description: "1 つまたは複数の Minecraft インスタンスを削除、残るもの、誤削除からの回復。" },
      "group-instances-and-favorites": { title: "インスタンスをグループ化・お気に入り", description: "名前付きグループと Favorites バーで大きな Library を整理。" },
      "open-instance-files": { title: "インスタンスのファイルを開く", description: "OS のファイルマネージャーでインスタンスフォルダーへ。ワールドのバックアップ、設定編集、スクリーンショット取り出し。" },
      "find-logs-and-crash-reports": { title: "ログとクラッシュレポートを見つける", description: "GDLauncher がログとクラッシュレポートを保存する場所、アプリ内表示、サポートへの添付方法。" },
      "import-an-instance": { title: "インスタンスをインポート", description: "CurseForge .zip、Modrinth .mrpack、GDL Share コード、または他のランチャーから取り込み。" },
      "export-an-instance": { title: "インスタンスをエクスポート", description: "GDLauncher インスタンスを CurseForge 形式 .zip としてエクスポート、共有・アーカイブ・アップロード可能。" },
      "share-an-instance": { title: "インスタンスを友達と共有", description: "GDLauncher Share コードを生成し、友達が同じ Mod、設定、リソースパックを入手。" },
      "allocate-more-ram": { title: "Minecraft への RAM 割り当てを増やす", description: "Minecraft により多くのメモリを割り当ててパフォーマンスを向上。" },
      "performance-mods": { title: "パフォーマンス Mod ガイド", description: "Minecraft の FPS を底上げする定番のパフォーマンス Mod。" },
      "enable-potato-mode": { title: "Potato PC Mode を有効化", description: "ランチャーのアニメーションをオフにして古いマシンの負荷を軽減。" },
      "allocate-server-ram": { title: "Minecraft サーバーへの RAM 割り当てを増やす", description: "プレイヤー数や Mod、チャンク負荷に対応するためサーバーのメモリを増やす。" },
      "setup-minecraft-server": { title: "Minecraft サーバーをセットアップ", description: "EULA 承認、ライブコンソール、メトリクスとともに GDLauncher からサーバーを作成・実行。" },
      "move-runtime-path": { title: "GDLauncher のデータを別ドライブに移動", description: "Runtime Path を変更して全インスタンスとアセットを別ドライブに移動。マイグレーションは自動。" },
      "backup-gdlauncher-data": { title: "PC 初期化前に GDLauncher をバックアップ", description: "フォーマットや PC 移行の前にインスタンス、アカウント、設定、ワールドを保存し、進行状況を失わずに復元する方法。" },
      "share-app-logs": { title: "GDLauncher アプリログを共有", description: "Windows、macOS、Linux でランチャーのアプリ全体のログを見つけ、Discord や GitHub のサポートリクエストに添付。" },
      "change-app-theme": { title: "GDLauncher のテーマを変更", description: "組み込みテーマとライブプレビューでランチャーの見た目を切替。" },
      "manage-microsoft-accounts": { title: "Microsoft アカウントを追加・切替・削除", description: "複数アカウントへのサインイン、Play が使うものの選択、期限切れトークンの更新、削除。" },
      "manage-java-versions": { title: "Java バージョンとインスタンス別オーバーライドを管理", description: "カスタム Java の追加、プロファイル作成、インスタンス別 Java オーバーライド設定。" },
      "curseforge-vs-modrinth": { title: "CurseForge vs Modrinth", description: "2 大 Minecraft Mod プラットフォームの違いを比較。" },
    },
    comparisons: {
      "prismlauncher": { title: "GDLauncher と Prism Launcher", description: "CurseForge 対応、自動化、Cloud Instance Sharing、UI を比較。" },
      "curseforge-app": { title: "GDLauncher と CurseForge App", description: "Modrinth と CurseForge を 1 つのランチャーに、Cloud Instance Sharingとサーバー管理も内蔵。" },
      "modrinth-app": { title: "GDLauncher と Modrinth App", description: "Modrinth と CurseForge を 1 つのランチャーで、加えて Cloud Instance Sharing とサーバー管理。" },
      "atlauncher": { title: "GDLauncher と ATLauncher", description: "長年の Java ランチャーと比較した最新の UI と深い CurseForge 連携。" },
      "multimc": { title: "GDLauncher と MultiMC", description: "MultiMC のミニマリズムに対する自動化、Modpack ワークフロー、現代的な UX。" },
      "ftb-app": { title: "GDLauncher と FTB App", description: "Modrinth 対応、Cloud Instance Sharing、サーバー管理を Feed The Beast 製ランチャーと比較。" },
      "tlauncher": { title: "GDLauncher と TLauncher", description: "公式 Microsoft サインインと、EULA に反する認証スキップ。後者で何を失うか。" },
    },
  },
  ko: {
    chrome: {
      pageTitle: "Minecraft 모딩 가이드 | GDLauncher",
      pageDescription: "Minecraft의 모드팩, 모드, 셰이더 등을 설치하는 방법을 안내합니다. 모드 적용 Minecraft를 시작하기 위한 단계별 튜토리얼.",
      breadcrumb: "가이드",
      h1Main: "Minecraft 모딩",
      h1Highlight: "가이드",
      intro: "모드, 모드팩, 셰이더 등 설치 방법을 배워 보세요. 모드 적용 Minecraft를 시작하기 위한 단계별 튜토리얼입니다.",
    },
    categories: {
      installation: "설치 가이드",
      modloaders: "모드 로더",
      instanceManagement: "인스턴스 관리",
      sharing: "공유와 이전",
      performance: "성능",
      servers: "서버",
      system: "시스템과 계정",
      platforms: "플랫폼 가이드",
      launcherComparisons: "런처 비교",
    },
    guides: {
      "install-modpack": { title: "모드팩 설치 방법", description: "GDLauncher로 Minecraft 모드팩을 몇 번의 클릭만으로 설치하는 방법." },
      "install-mods": { title: "모드 설치 방법", description: "CurseForge와 Modrinth에서 개별 Minecraft 모드를 찾아 설치하는 방법." },
      "install-shaders": { title: "셰이더 설치 방법", description: "아름다운 셰이더 팩으로 Minecraft 그래픽을 바꿔 보세요." },
      "install-resourcepacks": { title: "리소스 팩 설치 방법", description: "리소스 팩으로 Minecraft의 텍스처와 사운드를 커스터마이즈하세요." },
      "install-datapacks": { title: "데이터 팩 설치 방법", description: "모드 없이 새로운 레시피, 루트 테이블, 메카닉을 추가하세요." },
      "install-worlds": { title: "월드 설치 방법", description: "어드벤처 맵과 커스텀 월드 다운로드를 Minecraft에서 즐기세요." },
      "install-forge": { title: "Forge 설치 방법", description: "GDLauncher로 Minecraft Forge를 단계별로 설치하는 가이드." },
      "install-neoforge": { title: "NeoForge 설치 방법", description: "큰 콘텐츠 모드들이 모이는 현대적 Forge 포크 셋업." },
      "install-fabric": { title: "Fabric 설치 방법", description: "GDLauncher로 Minecraft에 Fabric 모드 로더를 설정하는 방법." },
      "install-quilt": { title: "Quilt 설치 방법", description: "대부분의 Fabric 모드에 더해 자체 모드까지 돌리는 Fabric 상위 호환 로더." },
      "switch-mod-loader": { title: "기존 인스턴스의 모드 로더 전환", description: "Forge에서 NeoForge로, Fabric에서 Quilt로, 인스턴스 재생성 없이 변경." },
      "update-modpack-version": { title: "모드팩을 새 버전으로 업데이트", description: "CurseForge/Modrinth 모드팩 인스턴스를 월드와 설정을 유지하며 업데이트." },
      "update-mods": { title: "인스턴스의 모드 업데이트", description: "개별 모드 또는 Update All로 인스턴스의 모드를 최신 상태로 유지." },
      "change-minecraft-version": { title: "인스턴스의 Minecraft 버전 바꾸기", description: "인스턴스 폴더를 유지하면서 MC 버전을 다른 버전으로 전환." },
      "change-instance-icon": { title: "인스턴스 아이콘 변경", description: "PNG/JPG 이미지로 각 Minecraft 인스턴스를 개별 커스터마이즈." },
      "duplicate-an-instance": { title: "인스턴스 복제", description: "기존 인스턴스의 전체 복사. 모드 변경 안전 테스트나 플레이 분기에." },
      "delete-instances-safely": { title: "인스턴스 안전하게 삭제", description: "단일 또는 다중 Minecraft 인스턴스 제거, 무엇이 남고, 실수 삭제 복구." },
      "group-instances-and-favorites": { title: "인스턴스 그룹화와 즐겨찾기", description: "명명된 그룹과 즐겨찾기 바로 큰 Library를 정리." },
      "open-instance-files": { title: "인스턴스 파일 열기", description: "OS 파일 매니저에서 인스턴스 폴더로 점프. 월드 백업, 설정 편집, 스크린샷 가져오기." },
      "find-logs-and-crash-reports": { title: "로그와 크래시 리포트 찾기", description: "GDLauncher가 로그와 크래시 리포트를 어디에 두는지, 앱에서 보고 지원에 첨부하는 법." },
      "import-an-instance": { title: "인스턴스 가져오기", description: "CurseForge .zip, Modrinth .mrpack, GDL 공유 코드 또는 다른 런처에서 가져오기." },
      "export-an-instance": { title: "인스턴스 내보내기", description: "GDLauncher 인스턴스를 CurseForge 형식 .zip으로 내보내 공유, 보관, 업로드 가능." },
      "share-an-instance": { title: "친구와 인스턴스 공유", description: "GDLauncher 공유 코드를 생성해 친구가 같은 모드, 설정, 리소스 팩을 설치하도록." },
      "allocate-more-ram": { title: "Minecraft에 RAM 더 할당하기", description: "Minecraft에 더 많은 메모리를 할당해 성능을 개선하세요." },
      "performance-mods": { title: "성능 모드 가이드", description: "필수 Minecraft 성능 모드로 FPS를 끌어올리세요." },
      "enable-potato-mode": { title: "Potato PC Mode 켜기", description: "런처 애니메이션을 꺼 오래된 머신의 부하를 줄입니다." },
      "allocate-server-ram": { title: "Minecraft 서버에 RAM 더 할당", description: "더 많은 플레이어, 모드, 청크 부하를 위해 서버에 메모리를 더 할당." },
      "setup-minecraft-server": { title: "Minecraft 서버 설정", description: "EULA 수락, 라이브 콘솔, 메트릭과 함께 GDLauncher에서 서버 생성·운영." },
      "move-runtime-path": { title: "GDLauncher 데이터를 다른 드라이브로", description: "Runtime Path를 변경해 모든 인스턴스와 에셋을 다른 드라이브로 이동. 마이그레이션 자동." },
      "backup-gdlauncher-data": { title: "PC 포맷 전에 GDLauncher 백업하기", description: "포맷이나 PC 교체 전에 인스턴스, 계정, 설정, 월드를 모두 저장하고 진행 상황을 잃지 않게 복원하는 방법." },
      "share-app-logs": { title: "GDLauncher 앱 로그 공유", description: "Windows, macOS, Linux에서 런처의 앱 전역 로그를 찾고 Discord나 GitHub 지원 요청에 첨부." },
      "change-app-theme": { title: "GDLauncher 테마 변경", description: "내장 테마와 라이브 프리뷰로 런처의 룩을 변경." },
      "manage-microsoft-accounts": { title: "Microsoft 계정 추가, 전환, 삭제", description: "여러 계정 로그인, Play가 쓸 계정 선택, 만료된 토큰 갱신, 계정 삭제." },
      "manage-java-versions": { title: "Java 버전과 인스턴스별 오버라이드 관리", description: "커스텀 Java 추가, 프로필 생성, 인스턴스별 Java 오버라이드 설정." },
      "curseforge-vs-modrinth": { title: "CurseForge vs Modrinth", description: "두 대표 Minecraft 모드 플랫폼의 차이점을 비교합니다." },
    },
    comparisons: {
      "prismlauncher": { title: "GDLauncher vs Prism Launcher", description: "CurseForge 지원, 자동화, Cloud Instance Sharing, 모던 UI 비교." },
      "curseforge-app": { title: "GDLauncher vs CurseForge App", description: "Modrinth와 CurseForge를 하나의 런처에서, Cloud Instance Sharing와 내장 서버 관리까지." },
      "modrinth-app": { title: "GDLauncher vs Modrinth App", description: "하나의 런처에서 Modrinth와 CurseForge, 그리고 Cloud Instance Sharing와 서버 관리." },
      "atlauncher": { title: "GDLauncher vs ATLauncher", description: "오래된 Java 런처와 비교한 모던 UI와 더 깊은 CurseForge 통합." },
      "multimc": { title: "GDLauncher vs MultiMC", description: "미니멀한 MultiMC와 비교한 자동화, 모드팩 워크플로, 모던 UX." },
      "ftb-app": { title: "GDLauncher vs FTB App", description: "Modrinth 지원, Cloud Instance Sharing, 서버 관리를 Feed The Beast 런처와 비교." },
      "tlauncher": { title: "GDLauncher vs TLauncher", description: "공식 Microsoft 로그인 대 EULA를 어기는 인증 우회, 그리고 그것을 쓸 때 잃는 것들." },
    },
  },
  de: {
    chrome: {
      pageTitle: "Minecraft Modding-Guides | GDLauncher",
      pageDescription: "Lerne, wie du Modpacks, Mods, Shader und mehr in Minecraft installierst. Schritt-für-Schritt-Tutorials für moddedes Minecraft.",
      breadcrumb: "Guides",
      h1Main: "Minecraft Modding-",
      h1Highlight: "Guides",
      intro: "Lerne, wie du Mods, Modpacks, Shader und mehr installierst. Schritt-für-Schritt-Anleitungen für den Einstieg ins moddede Minecraft.",
    },
    categories: {
      installation: "Installationsanleitungen",
      modloaders: "Mod-Loader",
      instanceManagement: "Instanz-Verwaltung",
      sharing: "Teilen & Migration",
      performance: "Performance",
      servers: "Server",
      system: "System & Accounts",
      platforms: "Plattform-Guides",
      launcherComparisons: "Launcher-Vergleiche",
    },
    guides: {
      "install-modpack": { title: "Modpacks installieren", description: "Lerne, wie du Minecraft-Modpacks mit GDLauncher in wenigen Klicks installierst." },
      "install-mods": { title: "Mods installieren", description: "Finde und installiere einzelne Minecraft-Mods aus CurseForge und Modrinth." },
      "install-shaders": { title: "Shader installieren", description: "Verändere die Grafik von Minecraft mit eindrucksvollen Shader-Paketen." },
      "install-resourcepacks": { title: "Resource Packs installieren", description: "Passe Texturen und Sounds von Minecraft mit Resource Packs an." },
      "install-datapacks": { title: "Data Packs installieren", description: "Füge ohne Mods neue Rezepte, Loot-Tabellen und Mechaniken hinzu." },
      "install-worlds": { title: "Welten installieren", description: "Spiele Adventure-Maps und Welt-Downloads in Minecraft." },
      "install-forge": { title: "Forge installieren", description: "Schritt-für-Schritt-Anleitung zur Installation von Minecraft Forge mit GDLauncher." },
      "install-neoforge": { title: "NeoForge installieren", description: "Richte den modernen Forge-Fork ein, auf den die meisten großen Content-Mods inzwischen zuerst setzen." },
      "install-fabric": { title: "Fabric installieren", description: "Lerne, wie du den Fabric-Mod-Loader für Minecraft mit GDLauncher einrichtest." },
      "install-quilt": { title: "Quilt installieren", description: "Installiere den Fabric-Superset-Loader, der die meisten Fabric-Mods plus eigene fährt." },
      "switch-mod-loader": { title: "Mod-Loader auf bestehender Instanz wechseln", description: "Von Forge auf NeoForge, Fabric auf Quilt usw., ohne die Instanz neu zu bauen." },
      "update-modpack-version": { title: "Modpack auf neue Version aktualisieren", description: "Bring eine CurseForge- oder Modrinth-Modpack-Instanz auf ein neues Release ohne Welten- oder Settingsverlust." },
      "update-mods": { title: "Mods einer Instanz aktualisieren", description: "Halte einzelne Mods aktuell, pro Mod oder mit Update All in einem Rutsch." },
      "change-minecraft-version": { title: "Minecraft-Version einer Instanz ändern", description: "Wechsle eine Custom-Instanz auf eine andere MC-Version, der Instanz-Ordner bleibt erhalten." },
      "change-instance-icon": { title: "Icon einer Instanz ändern", description: "Personalisier jede Minecraft-Instanz mit einem eigenen PNG- oder JPG-Icon." },
      "duplicate-an-instance": { title: "Eine Instanz duplizieren", description: "Vollständige Kopie einer Instanz erstellen, zum sicheren Testen von Mod-Änderungen oder zum Verzweigen eines Playthroughs." },
      "delete-instances-safely": { title: "Instanzen sicher löschen", description: "Eine oder mehrere Minecraft-Instanzen entfernen, was bleibt, und wie du nach versehentlichem Löschen wiederherstellst." },
      "group-instances-and-favorites": { title: "Instanzen gruppieren und favorisieren", description: "Eine große Library mit benannten Gruppen und einer Favoriten-Leiste ordnen." },
      "open-instance-files": { title: "Instanz-Dateien öffnen", description: "Im OS-Datei-Explorer in den Instanz-Ordner springen, für Welt-Backups, Config-Edits oder Screenshots." },
      "find-logs-and-crash-reports": { title: "Logs und Crash-Reports finden", description: "Wo GDLauncher Logs und Crash-Reports speichert, wie du sie in der App ansiehst und an Support-Requests anhängst." },
      "import-an-instance": { title: "Eine Instanz importieren", description: "Bestehende Minecraft-Instanz aus CurseForge-.zip, Modrinth-.mrpack, GDL-Share-Code oder einem anderen Launcher importieren." },
      "export-an-instance": { title: "Eine Instanz exportieren", description: "Eine GDLauncher-Instanz als CurseForge-format .zip exportieren, zum Teilen, Archivieren oder Hochladen." },
      "share-an-instance": { title: "Eine Instanz mit Freund teilen", description: "Einen GDLauncher-Share-Code generieren, damit ein Freund dieselben Mods, Configs und Resource Packs bekommt." },
      "allocate-more-ram": { title: "Mehr RAM für Minecraft zuweisen", description: "Verbessere die Performance, indem du Minecraft mehr Arbeitsspeicher zuweist." },
      "performance-mods": { title: "Performance-Mods-Guide", description: "Steigere deine FPS mit diesen essentiellen Minecraft-Performance-Mods." },
      "enable-potato-mode": { title: "Potato-PC-Mode aktivieren", description: "Animationen des Launchers ausschalten, um CPU- und GPU-Last auf älteren Maschinen zu reduzieren." },
      "allocate-server-ram": { title: "Mehr RAM für einen Minecraft-Server", description: "Mehr Speicher für mehr Spieler, mehr Mods und mehr Chunks auf deinem Server." },
      "setup-minecraft-server": { title: "Einen Minecraft-Server einrichten", description: "Server aus GDLauncher heraus erstellen und betreiben, mit EULA-Bestätigung, Live-Konsole und Metriken." },
      "move-runtime-path": { title: "GDLauncher-Daten auf anderes Laufwerk verschieben", description: "Den Runtime Path ändern, um alle Instanzen und Assets auf ein anderes Laufwerk zu verschieben, Migration läuft automatisch." },
      "backup-gdlauncher-data": { title: "GDLauncher vor dem Formatieren des PCs sichern", description: "Sichere alle Instanzen, Accounts, Einstellungen und Welten vor einer Formatierung oder einem PC-Wechsel und stelle alles ohne Fortschrittsverlust wieder her." },
      "share-app-logs": { title: "GDLauncher App-Logs teilen", description: "Die app-weiten Logs des Launchers auf Windows, macOS und Linux finden und an ein Discord-Ticket oder GitHub Issue anhängen." },
      "change-app-theme": { title: "GDLauncher-Theme ändern", description: "Anderes Aussehen für den Launcher mit eingebauten Themes und Live-Vorschau wählen." },
      "manage-microsoft-accounts": { title: "Microsoft-Accounts hinzufügen, wechseln, entfernen", description: "Bei mehreren Accounts anmelden, den aktiven für Play wählen, Tokens erneuern und Accounts entfernen." },
      "manage-java-versions": { title: "Java-Versionen und Per-Instanz-Override verwalten", description: "Eigene Java-Installationen hinzufügen, Profile anlegen und Per-Instanz-Java-Override setzen." },
      "curseforge-vs-modrinth": { title: "CurseForge vs Modrinth", description: "Vergleich der beiden großen Minecraft-Mod-Plattformen und ihrer Unterschiede." },
    },
    comparisons: {
      "prismlauncher": { title: "GDLauncher vs Prism Launcher", description: "Wie sich beide bei CurseForge-Support, Automatik, Instance-Share und modernem UI schlagen." },
      "curseforge-app": { title: "GDLauncher vs CurseForge App", description: "Modrinth und CurseForge in einem Launcher, plus Cloud Instance Sharing und integrierte Server-Verwaltung." },
      "modrinth-app": { title: "GDLauncher vs Modrinth App", description: "Modrinth und CurseForge in einem Launcher, plus Instance-Share und Server-Management." },
      "atlauncher": { title: "GDLauncher vs ATLauncher", description: "Modernes UI und tiefere CurseForge-Integration im Vergleich zum langjährigen Java-Launcher." },
      "multimc": { title: "GDLauncher vs MultiMC", description: "Automatik, Modpack-Workflows und modernes UX im Vergleich zu MultiMCs minimalistischer Linie." },
      "ftb-app": { title: "GDLauncher vs FTB App", description: "Modrinth, Cloud Instance Sharing und Server-Verwaltung im Vergleich zum Feed-The-Beast-Launcher." },
      "tlauncher": { title: "GDLauncher vs TLauncher", description: "Offizielle Microsoft-Anmeldung gegen einen EULA-widrigen Auth-Bypass, und was du dafür aufgibst." },
    },
  },
  fr: {
    chrome: {
      pageTitle: "Guides de modding Minecraft | GDLauncher",
      pageDescription: "Apprends à installer des modpacks, mods, shaders et plus encore pour Minecraft. Tutoriels pas-à-pas pour le Minecraft moddé.",
      breadcrumb: "Guides",
      h1Main: "Guides de modding",
      h1Highlight: "Minecraft",
      intro: "Apprends à installer mods, modpacks, shaders et plus encore. Tutoriels pas-à-pas pour démarrer avec Minecraft moddé.",
    },
    categories: {
      installation: "Guides d'installation",
      modloaders: "Mod loaders",
      instanceManagement: "Gestion des instances",
      sharing: "Partage et migration",
      performance: "Performance",
      servers: "Serveurs",
      system: "Système et comptes",
      platforms: "Guides plateformes",
      launcherComparisons: "Comparaisons de launchers",
    },
    guides: {
      "install-modpack": { title: "Comment installer des modpacks", description: "Apprends à installer des modpacks Minecraft avec GDLauncher en quelques clics." },
      "install-mods": { title: "Comment installer des mods", description: "Trouve et installe des mods Minecraft individuels depuis CurseForge et Modrinth." },
      "install-shaders": { title: "Comment installer des shaders", description: "Transforme la 3D de Minecraft avec de magnifiques shader packs." },
      "install-resourcepacks": { title: "Comment installer des resource packs", description: "Personnalise les textures et sons de Minecraft avec des resource packs." },
      "install-datapacks": { title: "Comment installer des data packs", description: "Ajoute des recettes, tables de butin et mécaniques sans mods." },
      "install-worlds": { title: "Comment installer des mondes", description: "Joue à des adventure maps et téléchargements de mondes dans Minecraft." },
      "install-forge": { title: "Comment installer Forge", description: "Guide pas-à-pas pour installer Minecraft Forge avec GDLauncher." },
      "install-neoforge": { title: "Comment installer NeoForge", description: "Configure le fork moderne de Forge que ciblent désormais la plupart des gros mods de contenu." },
      "install-fabric": { title: "Comment installer Fabric", description: "Apprends à configurer le mod loader Fabric pour Minecraft avec GDLauncher." },
      "install-quilt": { title: "Comment installer Quilt", description: "Installe le loader superset de Fabric, qui fait tourner la plupart des mods Fabric plus les siens." },
      "switch-mod-loader": { title: "Changer de mod loader sur une instance existante", description: "Passe de Forge à NeoForge, Fabric à Quilt, etc., sans recréer l'instance." },
      "update-modpack-version": { title: "Mettre à jour un modpack vers une nouvelle version", description: "Passe une instance modpack CurseForge ou Modrinth à une release plus récente sans perdre tes mondes." },
      "update-mods": { title: "Mettre à jour les mods d'une instance", description: "Garde tes mods à jour individuellement ou via Update All." },
      "change-minecraft-version": { title: "Changer la version de Minecraft d'une instance", description: "Bascule une instance custom sur une autre version de Minecraft, le dossier de l'instance est conservé." },
      "change-instance-icon": { title: "Changer l'icône d'une instance", description: "Personnalise chaque instance Minecraft avec ton propre PNG ou JPG." },
      "duplicate-an-instance": { title: "Dupliquer une instance", description: "Fais une copie complète d'une instance, utile pour tester des changements de mods ou brancher une partie." },
      "delete-instances-safely": { title: "Supprimer des instances en sécurité", description: "Retire une ou plusieurs instances Minecraft, ce qui est gardé, et comment récupérer après une suppression accidentelle." },
      "group-instances-and-favorites": { title: "Grouper des instances et marquer des favoris", description: "Organise une grande library avec des groupes nommés et une barre de favoris." },
      "open-instance-files": { title: "Ouvrir les fichiers d'une instance", description: "Saute au dossier de l'instance dans l'explorateur OS pour backup, édition de configs ou récupération de screenshots." },
      "find-logs-and-crash-reports": { title: "Trouver les logs et crash reports", description: "Où GDLauncher stocke logs et crash reports, comment les voir dans l'app et les joindre au support." },
      "import-an-instance": { title: "Importer une instance", description: "Importe une instance Minecraft depuis un .zip CurseForge, un .mrpack Modrinth, un code de partage GDL ou un autre launcher." },
      "export-an-instance": { title: "Exporter une instance", description: "Exporte une instance GDLauncher en .zip format CurseForge à partager, archiver ou uploader." },
      "share-an-instance": { title: "Partager une instance avec un ami", description: "Génère un code de partage GDLauncher pour qu'un ami installe les mêmes mods, configs et resource packs." },
      "allocate-more-ram": { title: "Allouer plus de RAM à Minecraft", description: "Améliore les performances en allouant plus de mémoire à Minecraft." },
      "performance-mods": { title: "Guide des mods de performance", description: "Booste tes FPS avec ces mods de performance essentiels pour Minecraft." },
      "enable-potato-mode": { title: "Activer Potato PC Mode", description: "Désactive les animations du launcher pour réduire la charge CPU et GPU sur machines anciennes." },
      "allocate-server-ram": { title: "Allouer plus de RAM à un serveur Minecraft", description: "Plus de mémoire pour plus de joueurs, plus de mods et plus de chunks." },
      "setup-minecraft-server": { title: "Monter un serveur Minecraft", description: "Crée et fais tourner un serveur depuis GDLauncher avec acceptation EULA, console live et métriques." },
      "move-runtime-path": { title: "Déplacer les données de GDLauncher sur un autre disque", description: "Change le Runtime Path pour déplacer instances et assets sur un autre disque, migration gérée automatiquement." },
      "backup-gdlauncher-data": { title: "Sauvegarder GDLauncher avant de formater son PC", description: "Mets en sécurité toutes tes instances, comptes, paramètres et mondes avant un format ou un changement de PC, puis restaure le tout sans perdre de progression." },
      "share-app-logs": { title: "Partager les logs de GDLauncher", description: "Trouve les logs globaux du launcher sur Windows, macOS et Linux et attache-les à un ticket Discord ou une issue GitHub." },
      "change-app-theme": { title: "Changer le thème de GDLauncher", description: "Choisis un look différent pour le launcher avec thèmes intégrés et preview live." },
      "manage-microsoft-accounts": { title: "Ajouter, changer, retirer des comptes Microsoft", description: "Connecte-toi à plusieurs comptes, choisis celui qu'utilise Play, rafraîchis les tokens et retire des comptes." },
      "manage-java-versions": { title: "Gérer les versions Java et l'override par instance", description: "Ajoute des installs Java custom, crée des profils et règle un override Java par instance." },
      "curseforge-vs-modrinth": { title: "CurseForge vs Modrinth", description: "Compare les deux grandes plateformes de mods Minecraft et leurs différences." },
    },
    comparisons: {
      "prismlauncher": { title: "GDLauncher vs Prism Launcher", description: "Comparaison sur le support CurseForge, l'automatisation, le partage cloud d'instance et l'UI moderne." },
      "curseforge-app": { title: "GDLauncher vs CurseForge App", description: "Modrinth et CurseForge dans un seul launcher, avec partage cloud d'instances et gestion de serveur intégrée." },
      "modrinth-app": { title: "GDLauncher vs Modrinth App", description: "Modrinth et CurseForge dans un seul launcher, avec partage cloud d'instance et gestion de serveur." },
      "atlauncher": { title: "GDLauncher vs ATLauncher", description: "UI moderne et intégration CurseForge plus poussée face au launcher Java historique." },
      "multimc": { title: "GDLauncher vs MultiMC", description: "Automatisation, flux modpack et UX moderne face aux racines minimalistes de MultiMC." },
      "ftb-app": { title: "GDLauncher vs FTB App", description: "Modrinth, partage cloud d'instance et gestion de serveur face au launcher Feed The Beast." },
      "tlauncher": { title: "GDLauncher vs TLauncher", description: "Connexion Microsoft officielle face à un bypass qui enfreint l'EULA, et ce que tu y perds." },
    },
  },
  es: {
    chrome: {
      pageTitle: "Guías de modding de Minecraft | GDLauncher",
      pageDescription: "Aprende a instalar modpacks, mods, shaders y más para Minecraft. Tutoriales paso a paso y guías para Minecraft con mods.",
      breadcrumb: "Guías",
      h1Main: "Guías de modding de",
      h1Highlight: "Minecraft",
      intro: "Aprende a instalar mods, modpacks, shaders y más. Tutoriales paso a paso para empezar con Minecraft con mods.",
    },
    categories: {
      installation: "Guías de instalación",
      modloaders: "Mod loaders",
      instanceManagement: "Gestión de instancias",
      sharing: "Compartir y migrar",
      performance: "Rendimiento",
      servers: "Servidores",
      system: "Sistema y cuentas",
      platforms: "Guías de plataformas",
      launcherComparisons: "Comparativas de launchers",
    },
    guides: {
      "install-modpack": { title: "Cómo instalar modpacks", description: "Aprende a instalar modpacks de Minecraft con GDLauncher en unos pocos clics." },
      "install-mods": { title: "Cómo instalar mods", description: "Encuentra e instala mods individuales de Minecraft desde CurseForge y Modrinth." },
      "install-shaders": { title: "Cómo instalar shaders", description: "Transforma los gráficos de Minecraft con preciosos shader packs." },
      "install-resourcepacks": { title: "Cómo instalar resource packs", description: "Personaliza las texturas y los sonidos de Minecraft con resource packs." },
      "install-datapacks": { title: "Cómo instalar data packs", description: "Añade nuevas recetas, tablas de loot y mecánicas sin mods." },
      "install-worlds": { title: "Cómo instalar mundos", description: "Juega mapas de aventura y descargas de mundos personalizados en Minecraft." },
      "install-forge": { title: "Cómo instalar Forge", description: "Guía paso a paso para instalar Minecraft Forge con GDLauncher." },
      "install-neoforge": { title: "Cómo instalar NeoForge", description: "Configura el fork moderno de Forge al que apuntan ya la mayoría de los grandes mods de contenido." },
      "install-fabric": { title: "Cómo instalar Fabric", description: "Aprende a configurar el mod loader Fabric en Minecraft con GDLauncher." },
      "install-quilt": { title: "Cómo instalar Quilt", description: "Instala el loader superset de Fabric, que ejecuta la mayoría de mods Fabric más los suyos propios." },
      "switch-mod-loader": { title: "Cambiar el mod loader en una instancia existente", description: "Cambia Forge a NeoForge, Fabric a Quilt, etc., sin recrear la instancia." },
      "update-modpack-version": { title: "Actualizar un modpack a una nueva versión", description: "Pasa una instancia modpack de CurseForge o Modrinth a una release más nueva sin perder mundos ni ajustes." },
      "update-mods": { title: "Actualizar los mods de una instancia", description: "Mantén tus mods al día con la opción individual o Update All." },
      "change-minecraft-version": { title: "Cambiar la versión de Minecraft de una instancia", description: "Cambia una instancia custom a otra versión de Minecraft preservando la carpeta de la instancia." },
      "change-instance-icon": { title: "Cambiar el icono de una instancia", description: "Personaliza cada instancia de Minecraft con tu propio PNG o JPG." },
      "duplicate-an-instance": { title: "Duplicar una instancia", description: "Haz una copia completa de una instancia, útil para probar cambios de mods con seguridad o bifurcar una partida." },
      "delete-instances-safely": { title: "Borrar instancias con seguridad", description: "Elimina una o varias instancias Minecraft, qué se queda y cómo recuperar de un borrado accidental." },
      "group-instances-and-favorites": { title: "Agrupar instancias y marcar favoritas", description: "Organiza una library grande con grupos nombrados y barra de favoritos." },
      "open-instance-files": { title: "Abrir los archivos de una instancia", description: "Salta a la carpeta de instancia en el explorador del SO para backup de mundos, edición de configs o capturas." },
      "find-logs-and-crash-reports": { title: "Encontrar logs y crash reports", description: "Dónde GDLauncher guarda logs y crash reports, cómo verlos en la app y adjuntarlos a soporte." },
      "import-an-instance": { title: "Importar una instancia", description: "Importa una instancia Minecraft desde .zip CurseForge, .mrpack Modrinth, código de compartición GDL u otro launcher." },
      "export-an-instance": { title: "Exportar una instancia", description: "Exporta una instancia GDLauncher como .zip en formato CurseForge para compartir, archivar o subir." },
      "share-an-instance": { title: "Compartir una instancia con un amigo", description: "Genera un código de compartición GDLauncher para que un amigo instale los mismos mods, configs y resource packs." },
      "allocate-more-ram": { title: "Asignar más RAM a Minecraft", description: "Mejora el rendimiento asignando más memoria a Minecraft." },
      "performance-mods": { title: "Guía de mods de rendimiento", description: "Sube tus FPS con estos mods esenciales de rendimiento para Minecraft." },
      "enable-potato-mode": { title: "Activar Potato PC Mode", description: "Apaga las animaciones del launcher para reducir carga de CPU y GPU en máquinas más antiguas." },
      "allocate-server-ram": { title: "Asignar más RAM a un servidor de Minecraft", description: "Más memoria para más jugadores, más mods y más carga de chunks." },
      "setup-minecraft-server": { title: "Montar un servidor de Minecraft", description: "Crea y ejecuta un servidor desde GDLauncher con aceptación de EULA, consola en vivo y métricas." },
      "move-runtime-path": { title: "Mover los datos de GDLauncher a otro disco", description: "Cambia el Runtime Path para mover instancias y assets a otro disco, migración automática." },
      "backup-gdlauncher-data": { title: "Hacer copia de seguridad de GDLauncher antes de formatear el PC", description: "Guarda todas tus instancias, cuentas, ajustes y mundos antes de formatear o cambiar de PC, y restáuralos sin perder progreso." },
      "share-app-logs": { title: "Compartir logs de GDLauncher", description: "Encuentra los logs de app del launcher en Windows, macOS y Linux y adjúntalos a un ticket de Discord o issue de GitHub." },
      "change-app-theme": { title: "Cambiar el tema de GDLauncher", description: "Elige otro look para el launcher con temas incorporados y vista previa en vivo." },
      "manage-microsoft-accounts": { title: "Añadir, cambiar y eliminar cuentas Microsoft", description: "Inicia sesión en varias cuentas, elige cuál usa Play, refresca tokens y elimina cuentas." },
      "manage-java-versions": { title: "Gestionar versiones de Java y override por instancia", description: "Añade Java custom, crea perfiles y configura un override de Java por instancia." },
      "curseforge-vs-modrinth": { title: "CurseForge vs Modrinth", description: "Compara las dos grandes plataformas de mods de Minecraft y sus diferencias." },
    },
    comparisons: {
      "prismlauncher": { title: "GDLauncher vs Prism Launcher", description: "Comparativa de soporte CurseForge, automatización, share de instancia y UI moderna." },
      "curseforge-app": { title: "GDLauncher vs CurseForge App", description: "Modrinth y CurseForge en un solo launcher, con compartir instancias en la nube y gestión de servidor integrada." },
      "modrinth-app": { title: "GDLauncher vs Modrinth App", description: "Modrinth y CurseForge en un solo launcher, con share de instancia y gestión de servidor." },
      "atlauncher": { title: "GDLauncher vs ATLauncher", description: "UI moderna e integración con CurseForge más profunda frente al longevo launcher Java." },
      "multimc": { title: "GDLauncher vs MultiMC", description: "Automatización, flujo de modpack y UX moderna frente a la línea minimalista de MultiMC." },
      "ftb-app": { title: "GDLauncher vs FTB App", description: "Modrinth, compartir instancia en la nube y gestión de servidor frente al launcher de Feed The Beast." },
      "tlauncher": { title: "GDLauncher vs TLauncher", description: "Inicio de sesión Microsoft oficial frente a un bypass que va contra el EULA, y qué pierdes al usarlo." },
    },
  },
  "pt-BR": {
    chrome: {
      pageTitle: "Guias de modding do Minecraft | GDLauncher",
      pageDescription: "Aprenda a instalar modpacks, mods, shaders e mais para o Minecraft. Tutoriais passo a passo e guias para Minecraft com mods.",
      breadcrumb: "Guias",
      h1Main: "Guias de modding do",
      h1Highlight: "Minecraft",
      intro: "Aprenda a instalar mods, modpacks, shaders e mais. Tutoriais passo a passo para começar no Minecraft com mods.",
    },
    categories: {
      installation: "Guias de instalação",
      modloaders: "Mod loaders",
      instanceManagement: "Gerenciamento de instâncias",
      sharing: "Compartilhar e migrar",
      performance: "Performance",
      servers: "Servidores",
      system: "Sistema e contas",
      platforms: "Guias de plataformas",
      launcherComparisons: "Comparações de launchers",
    },
    guides: {
      "install-modpack": { title: "Como instalar modpacks", description: "Aprenda a instalar modpacks de Minecraft com o GDLauncher em poucos cliques." },
      "install-mods": { title: "Como instalar mods", description: "Encontre e instale mods individuais de Minecraft no CurseForge e no Modrinth." },
      "install-shaders": { title: "Como instalar shaders", description: "Transforme os gráficos do Minecraft com belíssimos shader packs." },
      "install-resourcepacks": { title: "Como instalar resource packs", description: "Personalize texturas e sons do Minecraft com resource packs." },
      "install-datapacks": { title: "Como instalar data packs", description: "Adicione novas receitas, loot tables e mecânicas sem mods." },
      "install-worlds": { title: "Como instalar mundos", description: "Jogue adventure maps e downloads de mundos personalizados no Minecraft." },
      "install-forge": { title: "Como instalar o Forge", description: "Guia passo a passo para instalar o Minecraft Forge com o GDLauncher." },
      "install-neoforge": { title: "Como instalar o NeoForge", description: "Configure o fork moderno do Forge para o qual a maioria dos grandes mods de conteúdo já está mirando." },
      "install-fabric": { title: "Como instalar o Fabric", description: "Aprenda a configurar o mod loader Fabric no Minecraft com o GDLauncher." },
      "install-quilt": { title: "Como instalar o Quilt", description: "Instale o loader superset do Fabric, que roda a maioria dos mods Fabric mais os próprios." },
      "switch-mod-loader": { title: "Trocar o mod loader em uma instância existente", description: "Mude Forge pra NeoForge, Fabric pra Quilt, etc., sem recriar a instância." },
      "update-modpack-version": { title: "Atualizar um modpack para uma versão mais nova", description: "Mova uma instância de modpack CurseForge ou Modrinth para uma release mais nova sem perder mundos ou configs." },
      "update-mods": { title: "Atualizar mods de uma instância", description: "Mantenha seus mods em dia individualmente ou com Update All." },
      "change-minecraft-version": { title: "Mudar a versão do Minecraft de uma instância", description: "Troque uma instância custom para outra versão do Minecraft preservando a pasta da instância." },
      "change-instance-icon": { title: "Mudar o ícone de uma instância", description: "Personalize cada instância do Minecraft com seu próprio PNG ou JPG." },
      "duplicate-an-instance": { title: "Duplicar uma instância", description: "Faça uma cópia completa de uma instância, útil pra testar mudanças de mod com segurança ou bifurcar uma campanha." },
      "delete-instances-safely": { title: "Apagar instâncias com segurança", description: "Remova uma ou várias instâncias do Minecraft, o que fica, e como recuperar de uma exclusão acidental." },
      "group-instances-and-favorites": { title: "Agrupar instâncias e marcar favoritas", description: "Organize uma Library grande com grupos nomeados e uma barra de favoritos." },
      "open-instance-files": { title: "Abrir os arquivos de uma instância", description: "Pule pra pasta da instância no gerenciador do SO pra backup de mundos, edição de configs ou prints." },
      "find-logs-and-crash-reports": { title: "Encontrar logs e crash reports", description: "Onde o GDLauncher guarda logs e crash reports, como ver na app e anexar a um suporte." },
      "import-an-instance": { title: "Importar uma instância", description: "Importe uma instância Minecraft do .zip CurseForge, .mrpack Modrinth, código de compartilhamento GDL ou outro launcher." },
      "export-an-instance": { title: "Exportar uma instância", description: "Exporte uma instância GDLauncher como .zip em formato CurseForge pra compartilhar, arquivar ou enviar." },
      "share-an-instance": { title: "Compartilhar uma instância com um amigo", description: "Gere um código de compartilhamento GDLauncher pra um amigo instalar os mesmos mods, configs e resource packs." },
      "allocate-more-ram": { title: "Alocar mais RAM para o Minecraft", description: "Melhore a performance alocando mais memória para o Minecraft." },
      "performance-mods": { title: "Guia de mods de performance", description: "Aumente seu FPS com estes mods de performance essenciais para o Minecraft." },
      "enable-potato-mode": { title: "Ativar Potato PC Mode", description: "Desligue as animações do launcher pra reduzir carga de CPU e GPU em máquinas mais antigas." },
      "allocate-server-ram": { title: "Alocar mais RAM para um servidor de Minecraft", description: "Mais memória para mais jogadores, mais mods e mais carga de chunks." },
      "setup-minecraft-server": { title: "Montar um servidor Minecraft", description: "Crie e rode um servidor direto do GDLauncher com aceitação de EULA, console ao vivo e métricas." },
      "move-runtime-path": { title: "Mover dados do GDLauncher pra outro drive", description: "Mude o Runtime Path pra mover instâncias e assets pra outro drive, migração automática." },
      "backup-gdlauncher-data": { title: "Fazer backup do GDLauncher antes de formatar o PC", description: "Salve todas as suas instâncias, contas, configurações e mundos antes de formatar ou trocar de PC, depois restaure tudo sem perder progresso." },
      "share-app-logs": { title: "Compartilhar logs do GDLauncher", description: "Ache os logs do app do launcher no Windows, macOS e Linux e anexe num ticket do Discord ou issue do GitHub." },
      "change-app-theme": { title: "Mudar o tema do GDLauncher", description: "Escolha outro visual pro launcher com temas internos e preview ao vivo." },
      "manage-microsoft-accounts": { title: "Adicionar, trocar e remover contas Microsoft", description: "Entre em várias contas, escolha qual o Play usa, renove tokens expirados e remova contas." },
      "manage-java-versions": { title: "Gerenciar versões de Java e override por instância", description: "Adicione Java customizado, crie perfis e configure override de Java por instância." },
      "curseforge-vs-modrinth": { title: "CurseForge vs Modrinth", description: "Compare as duas principais plataformas de mods do Minecraft e suas diferenças." },
    },
    comparisons: {
      "prismlauncher": { title: "GDLauncher vs Prism Launcher", description: "Como os dois se comparam em suporte ao CurseForge, automação, share de instância e UI moderna." },
      "curseforge-app": { title: "GDLauncher vs CurseForge App", description: "Modrinth e CurseForge em um único launcher, com compartilhamento de instâncias na nuvem e gerenciamento de servidor embutido." },
      "modrinth-app": { title: "GDLauncher vs Modrinth App", description: "Modrinth e CurseForge no mesmo launcher, com share de instância e gerenciamento de servidor." },
      "atlauncher": { title: "GDLauncher vs ATLauncher", description: "UI moderna e integração mais profunda com o CurseForge contra o veterano launcher em Java." },
      "multimc": { title: "GDLauncher vs MultiMC", description: "Automação, fluxo de modpack e UX moderna em comparação com as raízes minimalistas do MultiMC." },
      "ftb-app": { title: "GDLauncher vs FTB App", description: "Modrinth, compartilhar instância na nuvem e gerenciamento de servidor no comparativo com o launcher do Feed The Beast." },
      "tlauncher": { title: "GDLauncher vs TLauncher", description: "Login Microsoft oficial contra um bypass que vai contra o EULA, e o que você perde ao usar." },
    },
  },
  it: {
    chrome: {
      pageTitle: "Guide al modding di Minecraft | GDLauncher",
      pageDescription: "Impara a installare modpack, mod, shader e altro per Minecraft. Tutorial passo passo e guide per Minecraft moddato.",
      breadcrumb: "Guide",
      h1Main: "Guide al modding di",
      h1Highlight: "Minecraft",
      intro: "Impara a installare mod, modpack, shader e altro. Tutorial passo passo per iniziare con Minecraft moddato.",
    },
    categories: {
      installation: "Guide all'installazione",
      modloaders: "Mod loader",
      instanceManagement: "Gestione istanze",
      sharing: "Condivisione e migrazione",
      performance: "Prestazioni",
      servers: "Server",
      system: "Sistema e account",
      platforms: "Guide sulle piattaforme",
      launcherComparisons: "Confronti tra launcher",
    },
    guides: {
      "install-modpack": { title: "Come installare i modpack", description: "Impara a installare modpack di Minecraft con GDLauncher in pochi clic." },
      "install-mods": { title: "Come installare le mod", description: "Trova e installa singole mod di Minecraft da CurseForge e Modrinth." },
      "install-shaders": { title: "Come installare le shader", description: "Trasforma la grafica di Minecraft con bellissimi shader pack." },
      "install-resourcepacks": { title: "Come installare i resource pack", description: "Personalizza texture e suoni di Minecraft con i resource pack." },
      "install-datapacks": { title: "Come installare i data pack", description: "Aggiungi nuove ricette, loot table e meccaniche senza mod." },
      "install-worlds": { title: "Come installare le mappe", description: "Gioca adventure map e mappe personalizzate in Minecraft." },
      "install-forge": { title: "Come installare Forge", description: "Guida passo passo per installare Minecraft Forge con GDLauncher." },
      "install-neoforge": { title: "Come installare NeoForge", description: "Configura il fork moderno di Forge su cui ormai punta la maggior parte delle grandi Mod di contenuto." },
      "install-fabric": { title: "Come installare Fabric", description: "Impara a configurare il mod loader Fabric per Minecraft con GDLauncher." },
      "install-quilt": { title: "Come installare Quilt", description: "Installa il loader superset di Fabric, che esegue la maggior parte delle Mod Fabric più i propri." },
      "switch-mod-loader": { title: "Cambia mod loader su un'istanza esistente", description: "Passa da Forge a NeoForge, da Fabric a Quilt o qualsiasi altra combinazione senza ricreare l'istanza." },
      "update-modpack-version": { title: "Aggiornare un modpack a una nuova versione", description: "Sposta un'istanza modpack CurseForge o Modrinth a una release più recente senza perdere mappe o impostazioni." },
      "update-mods": { title: "Aggiornare le mod di un'istanza", description: "Tieni le tue mod aggiornate singolarmente o con Update All." },
      "change-minecraft-version": { title: "Cambia la versione Minecraft di un'istanza", description: "Cambia un'istanza personalizzata a una versione di Minecraft diversa mantenendo intatta la cartella dell'istanza." },
      "change-instance-icon": { title: "Cambia l'icona di un'istanza", description: "Personalizza ogni istanza di Minecraft con un'icona PNG o JPG dal tuo computer." },
      "duplicate-an-instance": { title: "Duplicare un'istanza", description: "Fai una copia completa di un'istanza esistente, utile per testare modifiche alle mod in sicurezza o ramificare una partita." },
      "delete-instances-safely": { title: "Eliminare istanze in sicurezza", description: "Rimuovi una o più istanze Minecraft: cosa resta e come recuperare da una cancellazione accidentale." },
      "group-instances-and-favorites": { title: "Raggruppare istanze e segnare i preferiti", description: "Organizza una library grande con gruppi nominati e una barra dei preferiti." },
      "open-instance-files": { title: "Aprire i file di un'istanza", description: "Salta alla cartella dell'istanza nel file manager del sistema per backup delle mappe, modifiche ai config o screenshot." },
      "find-logs-and-crash-reports": { title: "Trovare log e crash report", description: "Dove GDLauncher salva log e crash report, come visualizzarli in-app e come allegarli a una richiesta di supporto." },
      "import-an-instance": { title: "Importare un'istanza", description: "Importa un'istanza Minecraft esistente da uno .zip CurseForge, .mrpack Modrinth, codice GDL Share o un altro launcher." },
      "export-an-instance": { title: "Esportare un'istanza", description: "Esporta un'istanza GDLauncher come .zip in formato CurseForge da condividere, archiviare o caricare." },
      "share-an-instance": { title: "Condividere un'istanza con un amico", description: "Genera un codice GDLauncher Share così un amico può installare le stesse mod, config e resource pack che hai tu." },
      "allocate-more-ram": { title: "Allocare più RAM a Minecraft", description: "Migliora le prestazioni assegnando più memoria a Minecraft." },
      "performance-mods": { title: "Guida alle mod di performance", description: "Aumenta gli FPS con queste mod di performance essenziali per Minecraft." },
      "enable-potato-mode": { title: "Attivare il Potato PC Mode", description: "Disattiva le animazioni del launcher per ridurre il carico su CPU e GPU su macchine più vecchie." },
      "allocate-server-ram": { title: "Allocare più RAM a un server Minecraft", description: "Aumenta la memoria disponibile per un server Minecraft per gestire più giocatori, mod e carico di chunk." },
      "setup-minecraft-server": { title: "Configurare un server Minecraft", description: "Crea e avvia un server Minecraft da GDLauncher con accettazione dell'EULA, console live e metriche." },
      "move-runtime-path": { title: "Spostare i dati di GDLauncher su un altro disco", description: "Cambia il runtime path per spostare tutte le istanze e gli asset su un disco diverso, migrazione automatica." },
      "backup-gdlauncher-data": { title: "Fare il backup di GDLauncher prima di formattare il PC", description: "Salva tutte le istanze, gli account, le impostazioni e i mondi prima di formattare o cambiare PC, poi ripristina tutto senza perdere progressi." },
      "share-app-logs": { title: "Condividere i log dell'app GDLauncher", description: "Trova i log a livello app del launcher su Windows, macOS e Linux e allegali a un ticket Discord o issue GitHub." },
      "change-app-theme": { title: "Cambiare il tema di GDLauncher", description: "Scegli un aspetto diverso per il launcher con temi integrati e anteprima live." },
      "manage-microsoft-accounts": { title: "Aggiungere, cambiare e rimuovere account Microsoft", description: "Accedi a più account, scegli quello usato da Play, aggiorna i token scaduti e rimuovi account." },
      "manage-java-versions": { title: "Gestire le versioni Java e l'override per istanza", description: "Aggiungi installazioni Java personalizzate, crea profili e imposta un override Java per ogni istanza." },
      "curseforge-vs-modrinth": { title: "CurseForge vs Modrinth", description: "Confronta le due principali piattaforme di mod per Minecraft e le loro differenze." },
    },
    comparisons: {
      "prismlauncher": { title: "GDLauncher vs Prism Launcher", description: "Come si confrontano i due su supporto CurseForge, automazione, condivisione cloud istanze e UI moderna." },
      "curseforge-app": { title: "GDLauncher vs CurseForge App", description: "Modrinth e CurseForge in un solo launcher, con condivisione cloud delle istanze e gestione server integrata." },
      "modrinth-app": { title: "GDLauncher vs Modrinth App", description: "Modrinth e CurseForge in un solo launcher, più condivisione cloud istanze e gestione server." },
      "atlauncher": { title: "GDLauncher vs ATLauncher", description: "UI moderna e integrazione CurseForge più profonda a confronto col veterano launcher Java." },
      "multimc": { title: "GDLauncher vs MultiMC", description: "Automazione, workflow modpack e UX moderna a confronto con le radici minimaliste di MultiMC." },
      "ftb-app": { title: "GDLauncher vs FTB App", description: "Modrinth, condivisione cloud istanze e gestione server a confronto con il launcher Feed The Beast." },
      "tlauncher": { title: "GDLauncher vs TLauncher", description: "Login Microsoft ufficiale contro un bypass dell'autenticazione che viola l'EULA e cosa perdi a usarlo." },
    },
  },
}

export function getGuidesHubData(locale: Locale): {
  chrome: GuidesHubChrome
  guides: GuideEntry[]
  categoryOrder: string[]
} {
  const d = data[locale] ?? data.en
  const guides: GuideEntry[] = SLUG_ORDER.map(([slug, catKey]) => ({
    slug,
    title: d.guides[slug].title,
    description: d.guides[slug].description,
    category: d.categories[catKey],
  }))

  // Append launcher comparisons after the canonical guides. These live at
  // /vs/<slug>, outside the /guides/ tree, so we set `href` explicitly with
  // the locale-aware path.
  const comparisonCategory = d.categories.launcherComparisons
  for (const slug of COMPARISON_ORDER) {
    const meta = d.comparisons[slug]
    if (!meta) continue
    guides.push({
      slug,
      title: meta.title,
      description: meta.description,
      category: comparisonCategory,
      href: localizedPath(`/vs/${slug}`, locale),
    })
  }

  const categoryOrder: string[] = []
  for (const g of guides) {
    if (!categoryOrder.includes(g.category)) categoryOrder.push(g.category)
  }
  return { chrome: d.chrome, guides, categoryOrder }
}
