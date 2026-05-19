---
title: "모드 로더 비교: Forge / NeoForge / Fabric / Quilt"
description: "GDLauncher가 지원하는 4가지 Minecraft 모드 로더. 각각의 위치, 호환성, 어떤 모드와 모드팩에 어떤 걸 선택할지."
faq:
  - question: "Minecraft에서 어떤 모드 로더를 써야 하나요?"
    answer: "당신이 원하는 모드나 모드팩이 요구하는 것을 쓰면 대부분 결정됩니다. 처음부터 자유롭게 고르는 경우: 최신 버전의 성능/QoL 위주는 Fabric, 새로운 콘텐츠 위주 대형 모드는 NeoForge, 옛 모드팩과 가장 큰 누적 라이브러리는 Forge."
  - question: "Forge 모드는 Fabric에서 작동하나요?"
    answer: "안 됩니다. Forge와 Fabric 모드는 호환되지 않습니다. 한쪽용으로 작성된 모드는 다른 쪽에서 로드되지 않습니다. 많은 인기 모드는 각각의 빌드를 따로 제공하니, 모드 페이지에서 지원 로더와 버전을 확인하세요."
  - question: "NeoForge가 Forge를 대체하나요?"
    answer: "새로운 Minecraft 버전에서는 사실상 그렇습니다. NeoForge는 2023년 Forge에서 같은 API로 갈라져 나온 포크였지만, 이후 둘이 분기해서 요즘 모드는 둘 다에서 도는 빌드 대신 NeoForge 빌드를 따로 냅니다. 1.20.4 이후 많은 Forge 계열 모드가 NeoForge로 빌드됩니다. 1.20.1 이전 버전에서는 여전히 Forge가 표준입니다."
  - question: "Fabric 모드는 Quilt에서 돌아가나요?"
    answer: "대부분 그렇습니다. Quilt는 Fabric의 포크이며 Fabric 모드를 직접 실행합니다. 일부 Quilt 전용 모드는 Quilt API를 써서 Fabric에서는 안 됩니다. 가지고 있는 모드가 전부 Fabric이라면 어느 로더로도 같은 결과입니다."
  - question: "두 로더를 동시에 쓸 수 있나요?"
    answer: "같은 인스턴스 안에서는 안 됩니다. 인스턴스마다 로더는 하나. 둘 다 쓰고 싶다면 인스턴스 두 개를 만드세요. GDLauncher의 인스턴스 시스템이 정확히 이걸 위해 설계됐습니다: Forge 인스턴스 하나, Fabric 인스턴스 하나, 클릭으로 전환."
---

# 모드 로더 비교: Forge / NeoForge / Fabric / Quilt

## GDLauncher가 지원하는 4가지 모드 로더

GDLauncher는 Minecraft Java Edition의 주요 모드 로더 4종과 Vanilla(로더 없음)를 설치/실행할 수 있습니다. 커스텀 인스턴스를 만들 때 선택합니다. 모드팩 설치 시엔 팩 매니페스트가 지정한 로더가 적용됩니다.

### Forge

최초의 모드 로더(2011년 시작). Forge는 가장 큰 역사적 모드 라이브러리를 가지고 있으며, 특히 콘텐츠 중심 모드(Tinkers' Construct, Twilight Forest, 초기 버전의 Create 등)에 강합니다. 옛 모드팩들도 대부분 Forge를 타깃으로 합니다.

업데이트는 Fabric보다 느린 편. 새 MC 버전 대응이 몇 주에서 몇 달 뒤가 되기도 합니다.

### NeoForge

2023년 Forge 커뮤니티 분열로 만들어진 Forge 포크. Forge와 거의 같은 API 스타일(모드가 대체로 소스 호환)을 유지하면서 더 빠르게 배포되고, 많은 Forge 모드 개발이 이쪽으로 옮겨갔습니다.

Minecraft 1.20.4 이후로는 둘 중 NeoForge가 더 활발합니다. 많은 대형 모드가 Forge와 동등하게, 또는 Forge 대신 NeoForge 빌드를 제공합니다.

### Fabric

다른 설계 철학: 작고, 빠르고, 모듈러. 새 MC 버전이 나오면 거의 같은 날(때로는 몇 시간 안에) 대응합니다. 모드 생태계는 성능(Sodium, Lithium, FerriteCore), QoL(Mod Menu, Iris), 최신 고품질 콘텐츠 모드에 강합니다.

성능 우선이거나 최신 MC 버전을 즐기고 싶다면 Fabric이 답입니다.

### Quilt

2022년 Fabric에서 분리된 포크. 거버넌스가 다르고 추가 API가 있습니다. Quilt는 Fabric 모드를 직접 실행하므로 실용적인 차이는 작습니다. 특정 모드가 요구하면 Quilt, 아니면 Fabric으로도 같은 결과.

Quilt는 Fabric보다 작은 전용 생태계지만 Fabric 콘텐츠와 거의 완전 호환됩니다.

## 호환성 매트릭스

| 빌드 대상 | Forge에서 작동 | NeoForge | Fabric | Quilt |
|---|---|---|---|---|
| Forge | 예 | 경우에 따라 (초기 NeoForge는 새로 갈라진 포크라 수정 없이 Forge 모드를 돌릴 수 있었지만, 이후 API가 분기해서 현재 Forge 모드 대부분은 NeoForge 빌드가 필요) | 아니오 | 아니오 |
| NeoForge | 아니오 | 예 | 아니오 | 아니오 |
| Fabric | 아니오 | 아니오 | 예 | 예 |
| Quilt | 아니오 | 아니오 | Quilt-API 사용 모드는 불가, 그 외는 가능 | 예 |

크로스 로더 브리지는 실용적으로 존재하지 않습니다. `mods/`에 넣는 JAR은 인스턴스의 로더와 일치해야 합니다.

## 새 인스턴스에서 선택

대부분 모드나 모드팩이 결정해 줍니다:

- **CurseForge / Modrinth에서 모드팩 설치?** GDLauncher가 팩 매니페스트를 읽고 지정 로더를 설치합니다. 선택 여지 없음.
- **특정 한 모드 중심 커스텀 인스턴스?** 모드 페이지를 확인하세요. "Fabric 1.21.x"라면 Fabric 1.21.x 인스턴스를 만듭니다.
- **여러 모드를 묶어서?** 모드마다 지원 로더를 조사해 교집합을 선택. 성능 모드는 Fabric, 콘텐츠 모드는 Forge/NeoForge가 많습니다.

아무 제약 없이 추천을 받고 싶다면: 성능/시각적 완성도는 **Fabric**, 콘텐츠 위주 모드 서바이벌은 **NeoForge**.

## 기존 인스턴스의 로더 전환

GDLauncher는 인스턴스 생성 후에도 로더 변경을 허용합니다. 자세히는 [How to Switch Mod Loaders on an Existing Instance](/guides/switch-mod-loader). 요점: 인스턴스 오른쪽 클릭 → Edit → 다른 로더 선택. mods 폴더는 비워지지 않으니 이전 로더의 JAR이 남습니다. 실행 전에 호환되지 않는 것을 손으로 제거하세요.

## 로더 버전에 대한 메모

각 로더는 Minecraft와 독립된 버전 체계를 가집니다. "Forge"를 고르면 Forge 버전(MC 1.20.1엔 `47.2.0` 같은)도 함께 고릅니다. 모드들은 보통 "팩이 기대하는 메이저와 같으면" 작동하지만, 일부는 최소 로더 빌드를 요구합니다. CurseForge / Modrinth 페이지에서 확인하세요.
