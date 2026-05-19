import type { LocaleData } from "./vsData"

const ko: LocaleData = {
  chrome: {
    compareBreadcrumb: "비교",
    feature: "기능",
    tryGdl: "GDLauncher 사용해보기",
    seeAllComparisons: "모든 비교 보기",
    theVerdict: "결론",
  },
  hub: {
    pageTitle: "GDLauncher vs 다른 Minecraft 런처: 1:1 비교",
    pageDescription:
      "GDLauncher와 다른 인기 Minecraft 런처의 상세 비교: Prism Launcher, CurseForge App, Modrinth App, ATLauncher, MultiMC, FTB App, TLauncher.",
    h1: "GDLauncher는 어떻게 비교될까",
    intro:
      "어떤 Minecraft 런처를 고를지 고민 중인가요? GDLauncher가 주요 대안들과 기능별로 어떻게 비교되는지 정리했습니다. 우리는 만든 입장이라 편향이 있을 수밖에 없지만, 비교를 글로 남겨 두니 직접 판단해 주세요.",
    competitors: {
      prismlauncher: {
        blurb:
          "가볍고 오픈소스인 MultiMC 포크. GDLauncher와 사용성, 모드팩 지원을 비교합니다.",
      },
      "curseforge-app": {
        blurb:
          "CurseForge의 공식 런처. CurseForge 연동, Modrinth 지원, 내장 서버 관리를 비교합니다.",
      },
      "modrinth-app": {
        blurb:
          "Modrinth 전용 런처. GDLauncher는 Modrinth와 CurseForge를 한 곳에서 제공합니다.",
      },
      atlauncher: {
        blurb:
          "베테랑 모드팩 런처. UI, 성능, 플랫폼 지원을 나란히 비교합니다.",
      },
      multimc: {
        blurb:
          "가볍고 파워 유저 친화적인 런처. 자동화와 모드팩 워크플로의 차이.",
      },
      "ftb-app": {
        blurb:
          "Feed The Beast 공식의 FTB / CurseForge 팩 런처. Modrinth 지원, Cloud Instance Sharing, 서버 관리의 차이.",
      },
      tlauncher: {
        blurb:
          "Mojang 인증을 건너뛰는 런처. 그 방식이 EULA에 어긋나는 이유와 사용 시 잃게 되는 것들.",
      },
    },
  },
  comparisons: {
    prismlauncher: {
      title: "GDLauncher vs Prism Launcher",
      description:
        "GDLauncher와 Prism Launcher 상세 비교: 기능, 모드팩 지원, 성능, UI. 자신에게 맞는 Minecraft 런처를 찾아보세요.",
      intro:
        "Prism Launcher는 인기 있는 오픈소스 MultiMC 포크입니다. GDLauncher는 CurseForge와 Modrinth를 깊게 통합한 모던 런처입니다. 실제로 매일 쓰면서 중요해지는 부분에서 두 런처가 어떻게 다른지 봅니다.",
      rows: [
        {
          feature: "CurseForge 지원",
          gdl: "예",
          competitor: "부분 (우회 필요)",
          note: "mod 작성자가 서드파티 API 다운로드를 막아 둔 경우, Prism은 해당 파일을 브라우저에서 직접 받도록 안내합니다",
        },
        { feature: "Modrinth 지원", gdl: "예", competitor: "예" },
        { feature: "Java 자동 관리", gdl: "예", competitor: "예" },
        { feature: "모드 자동 업데이트", gdl: "예", competitor: "아니요 (수동 확인만)" },
        { feature: "모드팩 자동 업데이트", gdl: "예", competitor: "아니요 (수동 확인만)" },
        { feature: "멀티 인스턴스", gdl: "예", competitor: "예" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "예 (원클릭 코드, CF + MR 혼합 지원)",
          competitor: "아니요 (수동 내보내기, CF + MR 혼합 미지원)",
        },
        { feature: "서버 관리", gdl: "예 (내장)", competitor: "아니요" },
        { feature: "모던 UI", gdl: "예", competitor: "아니요" },
        {
          feature: "애드온 제작자 보상",
          gdl: "예",
          competitor: "아니요",
        },
        { feature: "GitHub에 소스 공개", gdl: "예", competitor: "예" },
        { feature: "가벼움 (RAM)", gdl: "아니요", competitor: "예" },
      ],
      verdict:
        "Prism은 가벼우면서 군더더기 없는 런처를 원하고 모드팩에 손이 좀 가더라도 괜찮다면 훌륭한 선택입니다. GDLauncher는 CurseForge와 Modrinth에서 원클릭 설치, Cloud Instance Sharing, 내장 서버 관리까지 앱을 떠나지 않고 끝내고 싶은 사람을 위한 런처입니다. 모드 Minecraft가 처음이거나 미니멀함보다 다듬어진 경험을 원한다면 GDLauncher가 더 쉬운 길입니다.",
      sections: [
        {
          heading: "모드팩 워크플로",
          paragraphs: [
            "Prism과 GDLauncher 모두 런처 안에서 CurseForge 팩을 직접 둘러보고 설치할 수 있어서, 일상적인 경험은 비슷합니다. 차이는 경계에서 나옵니다. 모드 제작자가 서드파티 API 접근을 막아 둔 파일이 있으면 Prism은 해당 파일들을 매번 브라우저에서 직접 받도록 안내합니다. GDLauncher는 CurseForge와의 파트너십으로 그런 파일까지 바로 가져오기 때문에, 차단된 모드를 포함한 팩이어도 원클릭 설치가 유지됩니다.",
            "Modrinth 팩은 두 런처에서 동일하게, 앱 내 브라우저에서 원클릭으로 설치됩니다.",
          ],
        },
        {
          heading: "UI와 발견성",
          paragraphs: [
            "Prism의 Qt 기반 UI는 기능적이지만 투박합니다. 메인 화면은 인스턴스 리스트입니다. GDLauncher의 UI는 모드팩 탐색·관리에 맞춰 설계되어 있고, 내장 브라우저, 인스턴스 그룹화, 드래그&드롭 정렬, 비주얼 카드를 제공합니다. 주관적인 부분이지만 스크린샷을 비교해 볼 가치가 있습니다.",
          ],
        },
        {
          heading: "Cloud Instance Sharing",
          paragraphs: [
            "GDLauncher에는 원클릭 Cloud Instance Sharing가 있습니다. 코드 하나만 붙여 넣으면 동일한 셋업이 그대로 들어옵니다. Prism은 파일로 인스턴스를 내보내고 들여오는 방식인데, 동작은 하지만 친구와 공유하기에는 매끄럽지 않습니다.",
          ],
        },
      ],
    },
    "curseforge-app": {
      title: "GDLauncher vs CurseForge App",
      description:
        "GDLauncher와 CurseForge App 비교: 기능, 광고, Modrinth 지원, 서버 관리. 모드 Minecraft를 즐기는 더 나은 방법.",
      intro:
        "CurseForge App은 CurseForge 콘텐츠 전용 공식 런처입니다. GDLauncher도 CurseForge와 통합되어 있고, 같은 브라우저에서 Modrinth, 두 플랫폼을 아우르는 Cloud Instance Sharing, 내장 서버 관리까지 함께 제공합니다. 정리해 보겠습니다.",
      rows: [
        {
          feature: "CurseForge 지원",
          gdl: "예",
          competitor: "예 (네이티브, 자사 앱)",
        },
        { feature: "Modrinth 지원", gdl: "예", competitor: "아니요" },
        { feature: "Java 자동 관리", gdl: "예", competitor: "예" },
        { feature: "모드 자동 업데이트", gdl: "예", competitor: "예 (확인 필요)" },
        { feature: "모드팩 자동 업데이트", gdl: "예", competitor: "예 (확인 필요)" },
        { feature: "멀티 인스턴스", gdl: "예", competitor: "예" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "예 (원클릭 코드, CF + MR 혼합 지원)",
          competitor: "예 (CurseForge 전용)",
        },
        { feature: "서버 관리", gdl: "예 (내장)", competitor: "아니요" },
        {
          feature: "앱 내 광고",
          gdl: "예 (앱 내 배너)",
          competitor: "예 (앱 내 배너)",
        },
        { feature: "GitHub에 소스 공개", gdl: "예", competitor: "아니요" },
        { feature: "애드온 제작자 보상", gdl: "예", competitor: "예" },
      ],
      verdict:
        "CurseForge 콘텐츠만 설치한다면 CurseForge App이 공식 선택지입니다. GDLauncher는 동일한 CurseForge 연동에 더해 같은 브라우저에서 Modrinth를 쓰고, CurseForge와 Modrinth가 섞인 설정을 그대로 들고 다닐 수 있는 Cloud Instance Sharing와 내장 서버 관리를 제공합니다.",
      sections: [
        {
          heading: "한 런처 안의 Modrinth",
          paragraphs: [
            "CurseForge App은 설계상 CurseForge 전용입니다. Modrinth는 Fabric 모드, 성능 모드, 셰이더를 중심으로 빠르게 성장하고 있고, 많은 제작자가 양쪽 플랫폼에 올리고 있습니다. GDLauncher의 내장 브라우저는 두 플랫폼을 한 번에 검색하므로 굳이 고를 필요가 없습니다.",
          ],
        },
        {
          heading: "서버 관리",
          paragraphs: [
            "GDLauncher에는 Minecraft 서버 관리가 내장되어 있습니다. Vanilla, Forge, Fabric, NeoForge, Quilt 서버를 만들고 싱글 인스턴스와 동일한 UI에서 관리할 수 있습니다. CurseForge App에는 서버 관리 기능이 없습니다.",
          ],
        },
        {
          heading: "Cloud Instance Sharing",
          paragraphs: [
            "두 런처 모두 친구와 셋업을 공유할 수 있습니다. CurseForge App은 모든 것을 CurseForge 생태계 안에서만 처리합니다. CurseForge 모드팩은 넘겨줄 수 있지만, CurseForge 모드와 Modrinth 모드가 섞인 셋업은 그대로 전달되지 않습니다. GDLauncher의 Cloud Instance Sharing는 혼합 케이스도 받습니다. 코드 하나만 붙여 넣으면 상대방은 두 플랫폼의 파일이 원본 CDN에서 다시 다운로드된, 당신의 정확한 인스턴스를 그대로 받습니다.",
          ],
        },
      ],
    },
    "modrinth-app": {
      title: "GDLauncher vs Modrinth App",
      description:
        "GDLauncher vs Modrinth App: 모드와 모드팩에 가장 좋은 Minecraft 런처는? 기능, 플랫폼, 생태계 지원 비교.",
      intro:
        "Modrinth App은 공식 Modrinth 런처이고 Modrinth 콘텐츠만 쓴다면 훌륭한 선택입니다. GDLauncher도 Modrinth와 통합되어 있고, 거기에 CurseForge, Cloud Instance Sharing, 서버 관리를 더합니다. 나란히 비교해 보겠습니다.",
      rows: [
        {
          feature: "CurseForge 지원",
          gdl: "예",
          competitor: "아니요",
        },
        {
          feature: "Modrinth 지원",
          gdl: "예",
          competitor: "예 (네이티브, 자사 앱)",
        },
        { feature: "Java 자동 관리", gdl: "예", competitor: "예" },
        { feature: "모드 자동 업데이트", gdl: "예", competitor: "예 (확인 필요)" },
        { feature: "모드팩 자동 업데이트", gdl: "예", competitor: "예 (확인 필요)" },
        { feature: "멀티 인스턴스", gdl: "예", competitor: "예" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "예 (원클릭 코드, CF + MR 혼합 지원)",
          competitor: "아니요 (수동 내보내기, Modrinth 전용)",
        },
        { feature: "서버 관리", gdl: "예 (내장)", competitor: "예 (Modrinth Hosting)" },
        { feature: "모던 UI", gdl: "예", competitor: "예" },
        { feature: "GitHub에 소스 공개", gdl: "예", competitor: "예" },
        { feature: "애드온 제작자 보상", gdl: "예", competitor: "예" },
        { feature: "가벼움", gdl: "중간", competitor: "중간" },
      ],
      verdict:
        "Modrinth App은 완전히 Modrinth 생태계 안에서 산다면 환상적입니다. 하지만 가장 인기 있는 모드팩(RLCraft, ATM10, DawnCraft, FTB 라인업)은 여전히 CurseForge 전용이고, 양쪽 플랫폼에 모두 올라오는 팩도 보통 CurseForge가 우선입니다. GDLauncher는 Modrinth와 CurseForge를 한 브라우저에 모아 주고, 친구와의 Cloud Instance Sharing와 내장 서버 관리도 제공합니다. 더 넓은 생태계를 원하면 GDLauncher, Modrinth만 집중적으로 쓰고 싶다면 Modrinth App을 고르세요.",
      sections: [
        {
          heading: "CurseForge 격차",
          paragraphs: [
            "가장 큰 차이는 단순합니다. Modrinth App은 CurseForge 콘텐츠를 설치하지 못합니다. Modrinth 전용 모드만 쓴다면 문제가 아닙니다. 하지만 CurseForge는 여전히 더 큰 모드팩 라이브러리와 다수의 오래된 Forge 모드를 단독 보유하고 있습니다. GDLauncher의 브라우저는 두 플랫폼을 한 검색에 모아 주므로 필요한 버전이 있는 쪽을 고르면 됩니다.",
          ],
        },
        {
          heading: "두 생태계 모두 훌륭함",
          paragraphs: [
            "Modrinth는 라이브러리는 작지만 더 빠르고 광고 없는 사이트, 그리고 모더 친화적인 API를 갖추고 있습니다. CurseForge는 카탈로그가 더 깊고 역사적인 팩이 많습니다. 인기 있는 모드는 대부분 이제 양쪽에 있습니다. GDLauncher의 전략은 사용자에게 선택을 강요하지 않고 두 곳을 네이티브로 지원하는 것입니다.",
          ],
        },
        {
          heading: "서버 관리와 Cloud Instance Sharing",
          paragraphs: [
            "Modrinth의 서버 관리는 유료 Modrinth Hosting 연동입니다. Modrinth에서 서버를 프로비저닝하고 앱에서 관리합니다. GDLauncher의 서버 관리는 로컬에서 동작합니다. 자신의 머신에 Vanilla / Forge / Fabric / NeoForge / Quilt 서버를 만들고, 라이브 콘솔, 플레이어 관리, 싱글플레이와 동일한 인스턴스 설정을 그대로 사용할 수 있습니다. 호스팅 요금은 없습니다.",
            "Cloud Instance Sharing는 Modrinth App에 없는 GDLauncher의 또 다른 기능입니다. 코드를 붙여 넣으면 CurseForge와 Modrinth를 한 번에 섞은 동일 셋업이 그대로 들어옵니다.",
          ],
        },
      ],
    },
    atlauncher: {
      title: "GDLauncher vs ATLauncher",
      description:
        "GDLauncher와 ATLauncher 상세 비교: UI, 모드팩 지원, 서버 관리, 개발 경험. 어느 쪽이 더 나은 Minecraft 런처인가?",
      intro:
        "ATLauncher는 오랜 기간 운영된 Java 기반 모드팩 런처로, 자체 ATLauncher 팩 생태계를 가지고 있습니다. GDLauncher는 더 새로운 Rust + Solid 대안으로, 모던 UI와 CurseForge / Modrinth 원클릭 설치를 제공합니다. 비교해 봅니다.",
      rows: [
        {
          feature: "CurseForge 지원",
          gdl: "예",
          competitor: "부분 (우회 필요)",
          note: "mod 작성자가 서드파티 API 다운로드를 막아 둔 경우, ATLauncher는 해당 파일을 브라우저에서 직접 받도록 안내합니다",
        },
        { feature: "Modrinth 지원", gdl: "예", competitor: "예" },
        { feature: "Java 자동 관리", gdl: "예", competitor: "예" },
        { feature: "모드 자동 업데이트", gdl: "예", competitor: "예 (확인 필요)" },
        { feature: "모드팩 자동 업데이트", gdl: "예", competitor: "예 (확인 필요)" },
        { feature: "멀티 인스턴스", gdl: "예", competitor: "예" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "예 (원클릭 코드, CF + MR 혼합 지원)",
          competitor: "아니요 (수동 내보내기, CF + MR 혼합 미지원)",
        },
        { feature: "서버 관리", gdl: "예 (내장)", competitor: "아니요" },
        {
          feature: "모던 UI",
          gdl: "예",
          competitor: "부분 (FlatLaf 적용 Java Swing)",
        },
        { feature: "애드온 제작자 보상", gdl: "예", competitor: "아니요" },
        { feature: "GitHub에 소스 공개", gdl: "예", competitor: "예" },
        {
          feature: "커스텀 모드팩 배포",
          gdl: "예 (Cloud Instance Sharing 원클릭 코드)",
          competitor: "예 (ATLauncher 팩)",
        },
      ],
      verdict:
        "ATLauncher는 ATLauncher가 직접 큐레이션한 팩 목록을 쓰고 싶거나 이미 그 워크플로에 익숙하다면 견고한 선택입니다. GDLauncher의 강점은 더 모던한 UI, 더 깊은 CurseForge 연동, Cloud Instance Sharing, 내장 서버 관리입니다. 2026년의 대부분 모드 Minecraft 플레이어에게는 GDLauncher의 경험이 모던 앱에 기대하는 모습에 더 가깝습니다.",
      sections: [
        {
          heading: "UI 세대 차이",
          paragraphs: [
            "ATLauncher는 Java Swing 위에 모던한 FlatLaf 룩앤필을 얹어 사용합니다. 기존의 클래식 Swing에 비하면 실질적인 진전이지만, 밀도, 모션, 플랫폼 감각 면에서는 여전히 네이티브 모던 런처에 못 미칩니다. GDLauncher는 Solid로 만들어졌고, UnoCSS 기반의 자체 디자인 시스템, 네이티브 같은 드래그&드롭, 애니메이션, 그룹화를 갖추고 있습니다.",
          ],
        },
        {
          heading: "CurseForge 연동",
          paragraphs: [
            "ATLauncher와 GDLauncher 둘 다 런처 안에서 CurseForge 팩을 검색하고 설치할 수 있어 일상 경험은 비슷합니다. 차이는 가장자리에서 드러납니다. mod 작성자가 자기 파일에 대한 서드파티 API 접근을 막아 둔 경우, ATLauncher는 차단된 링크마다 클릭해 브라우저에서 파일을 직접 받게 합니다. GDLauncher의 CurseForge 파트너십은 그런 파일도 직접 가져오므로, 차단된 mod가 포함된 팩이라도 원클릭 설치가 그대로 유지됩니다.",
          ],
        },
        {
          heading: "ATLauncher 팩 vs Cloud Instance Sharing",
          paragraphs: [
            "ATLauncher는 자체 팩 생태계를 호스팅합니다. GDLauncher는 그 분야에서 경쟁하지 않고, 대신 Cloud Instance Sharing를 통해 누구나 자신의 정확한 셋업(모드, 설정값, 옵션)을 단일 코드로 공유할 수 있게 합니다. 철학이 다른 거라, 본인과 친구들의 플레이 스타일에 맞는 쪽을 고르면 됩니다.",
          ],
        },
      ],
    },
    multimc: {
      title: "GDLauncher vs MultiMC",
      description:
        "GDLauncher와 MultiMC 상세 비교: 기능, 자동화, 모드팩 처리, 모던 UI. 당신에게 맞는 Minecraft 런처를 찾아보세요.",
      intro:
        "MultiMC는 멀티 인스턴스 Minecraft 런칭을 개척한 런처입니다. 다만 마지막 공식 릴리스는 2021년 12월의 0.6.14였고, 활발한 개발은 대부분 포크(특히 Prism Launcher)로 옮겨 갔습니다. GDLauncher는 자동화에 강한, 의견이 분명한 모던 런처입니다. 실용적인 비교를 보겠습니다.",
      rows: [
        {
          feature: "CurseForge 지원",
          gdl: "예",
          competitor: "아니요",
        },
        { feature: "Modrinth 지원", gdl: "예", competitor: "예" },
        { feature: "Java 자동 관리", gdl: "예", competitor: "아니요" },
        { feature: "모드 자동 업데이트", gdl: "예", competitor: "아니요" },
        { feature: "모드팩 자동 업데이트", gdl: "예", competitor: "아니요" },
        {
          feature: "멀티 인스턴스",
          gdl: "예",
          competitor: "예 (특기 분야)",
        },
        {
          feature: "Cloud Instance Sharing",
          gdl: "예 (원클릭 코드, CF + MR 혼합 지원)",
          competitor: "아니요 (수동 내보내기, CF + MR 혼합 미지원)",
        },
        { feature: "서버 관리", gdl: "예 (내장)", competitor: "아니요" },
        { feature: "모던 UI", gdl: "예", competitor: "아니요" },
        { feature: "애드온 제작자 보상", gdl: "예", competitor: "아니요" },
        { feature: "GitHub에 소스 공개", gdl: "예", competitor: "예" },
        { feature: "가벼움", gdl: "아니요", competitor: "예 (아주 가벼움)" },
      ],
      verdict:
        "MultiMC는 매우 작고 유연한 런처를 원하면서 Java 설정, 모드 관리, 업데이트를 직접 다루는 게 좋다면 훌륭한 선택입니다. GDLauncher는 이런 것들을 자동으로 처리받고 싶은 플레이어를 위한 런처로, 자동 Java, 자동 업데이트, 원클릭 설치, Cloud Instance Sharing, 서버 관리가 모두 포함되며, MultiMC가 개척한 멀티 인스턴스 워크플로 역시 유지합니다.",
      sections: [
        {
          heading: "자동화 vs 통제",
          paragraphs: [
            "MultiMC의 설계 철학은 \"사용자가 요청하지 않은 일은 하지 않는다\"입니다. 즉 Java 경로도, 버전도, 모드 관리도, 업데이트도 모두 직접 해야 합니다. 파워 유저는 이걸 좋아하지만, 신규 플레이어는 떠납니다.",
            "GDLauncher는 정반대 접근입니다. 인스턴스마다 필요한 것을 감지해 설치하고 최신 상태로 유지하지만, 원한다면 인스턴스 설정에서 동일한 항목을 모두 오버라이드할 수 있습니다. 기본값으로도 잘 돌고, 통제 옵션도 그대로 있습니다.",
          ],
        },
        {
          heading: "모드팩 처리",
          paragraphs: [
            "MultiMC에는 Modrinth 브라우저가 내장돼 있지만 CurseForge 연동은 없습니다. CurseForge 모드팩을 플레이하려면 zip 파일로 수동 임포트하거나 서드파티 도구로 매니페스트를 가져와야 합니다. GDLauncher의 브라우저는 CurseForge와 Modrinth를 나란히 보여주며 양쪽 모두 원클릭 설치를 지원합니다.",
          ],
        },
        {
          heading: "계보",
          paragraphs: [
            "MultiMC는 2021년 12월 이후로 새 릴리스를 내지 않았고, 프로젝트의 동력은 사실상 Prism Launcher와 다른 포크들로 옮겨 갔습니다. 오래 MultiMC를 써 왔고 워크플로를 잃지 않으면서 더 모던한 UI를 원한다면 Prism이 자연스러운 업그레이드 경로이고, GDLauncher는 더 큰 도약(더 많은 자동화, 더 적은 수동 단계)에 가깝습니다. 둘 다 써 보고 실제 사용 방식에 맞는 모델을 고르세요.",
          ],
        },
        {
          heading: "Cloud Instance Sharing",
          paragraphs: [
            "MultiMC에서 친구와 셋업을 공유하려면 인스턴스를 zip으로 내보내고 파일을 건네줘야 합니다. 동작은 하지만, 어딘가에 호스팅해야 하는 파일이고 받는 쪽도 똑같이 임포트해야 합니다. GDLauncher의 Cloud Instance Sharing는 이걸 짧은 코드 하나로 대체합니다. 코드를 붙여 넣으면 런처가 GDL 서비스에서 스냅샷을 가져오고 모드는 원본 CDN에서 다시 다운로드됩니다. 코드 하나로 같은 공유 안에 CurseForge + Modrinth 콘텐츠가 섞여 있어도 되고, zip 파일을 주고받을 필요도 없습니다.",
          ],
        },
      ],
    },
  },
}

export default ko
