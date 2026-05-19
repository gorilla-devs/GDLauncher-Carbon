---
title: "모드팩 매니페스트 포맷"
description: "GDLauncher가 읽고 쓰는 세 가지 모드팩 아카이브 안에 있는 것: CurseForge .zip, Modrinth .mrpack, 그리고 GDLauncher 자체의 해시 기반 .gdlpack. 필드별 레퍼런스, 횡 비교, 그리고 각 포맷이 설치 시점에 모드를 어떻게 해결하는지에 대한 설명."
faq:
  - question: "모드팩 매니페스트가 뭔가요?"
    answer: "모드팩 아카이브 안의 JSON 파일로, 팩 내용(Minecraft 버전, 모드 로더, 모드 목록, 인스턴스에 풀어 넣을 번들 파일(오버라이드))을 런처에 알려 줍니다. GDLauncher는 팩을 설치할 때 가장 먼저 매니페스트를 읽습니다."
  - question: "CurseForge, Modrinth, GDLauncher 팩 포맷의 차이는?"
    answer: "셋 다 JSON 매니페스트가 들어 있는 zip. CurseForge는 manifest.json을 쓰고 CurseForge 프로젝트/파일 ID로 참조. Modrinth는 modrinth.index.json을 쓰고 직접 다운로드 URL + 해시로 참조. GDLauncher는 gdlpack.json을 쓰고 콘텐츠 해시만(sha512 + sha1 + murmur2)으로 각 파일을 실제로 가진 플랫폼에 대해 해결합니다. 호환되지 않으며 런처는 아카이브 루트에 있는 JSON에 따라 처리합니다."
  - question: ".gdlpack은 무엇이고 왜 존재하나요?"
    answer: "GDLauncher 자체의 모드팩 포맷. 각 모드는 콘텐츠 해시로 식별되므로, 모드가 CurseForge에 있든 Modrinth에 있든 둘 다 있든 같은 팩이 동작하고 URL이 썩지도 않습니다. 옵션 기능(스킵 가능한 모드 + 설정 묶음)과 임베디드 아이콘도 1급 지원. 양쪽 모두 GDLauncher를 쓸 때 권장. 크로스 런처 배포에는 여전히 CurseForge .zip이 안전합니다."
  - question: "설치된 인스턴스의 매니페스트는 어디서 보나요?"
    answer: "GDLauncher는 설치 후 원본 아카이브를 보관하지 않습니다. 팩 내용은 인스턴스 폴더에 풀려 있습니다. GDLauncher가 사용하는 매니페스트는 런처 DB에 인스턴스 단위로 추적되며 파일로 열람할 수 없습니다. 설치 전에 보고 싶다면 .zip / .mrpack / .gdlpack을 아무 아카이브 도구로 열어 안의 JSON 파일을 직접 읽으세요."
  - question: "매니페스트를 편집할 수 있나요?"
    answer: "자신의 팩을 만든다면 아카이브 안의 매니페스트를 편집할 수 있습니다. 설치된 인스턴스의 매니페스트는 런처 UI를 통해 편집할 수 없습니다. 어떤 모드가 들어 있는지 바꾸려면 Addons 탭(잠긴 인스턴스라면 먼저 해제). 페어된 팩의 버전을 바꾸려면 인스턴스를 열고 Settings 탭에서 Change Modpack Version을 클릭."
  - question: "팩 용어의 'override'가 뭔가요?"
    answer: "모드가 아니라 팩이 사전 번들로 제공하는 파일. 기본 설정, 스크립트, 리소스 팩, 가끔 스타터 월드. 팩 설치 시 모드 위에 인스턴스 폴더로 풀어집니다. CurseForge는 overrides 필드로 나열, Modrinth는 env.client=required인 팩 파일을 사용, GDLPack은 overrides 폴더(매니페스트에서 설정 가능)를 사용."
---

# 모드팩 매니페스트 포맷

## 왜 알아야 하나

평소엔 모드팩 아카이브 안을 알 필요가 없습니다. 그러나 설치 중 문제가 생기면 "manifest", "invalid manifest format", "manifest missing field", "manifest version unsupported" 같은 메시지를 보게 되고, 그게 뭘 가리키는지 알고 싶어집니다. 공유용 팩을 만들거나, 같은 팩이 한 런처에서는 깨끗이 설치되는데 다른 런처에서는 망가지는 원인을 찾을 때도 유용합니다.

GDLauncher는 세 가지 포맷을 읽고 씁니다:

- **CurseForge** (`.zip`, 내부 `manifest.json`).
- **Modrinth** (`.mrpack`, 내부 `modrinth.index.json`).
- **GDLauncher** (`.gdlpack`, 내부 `gdlpack.json`).

셋 다 내부는 평범한 ZIP 아카이브. 확장자는 어떤 매니페스트 스키마를 기대해야 할지에 대한 힌트일 뿐.

## 모드팩 아카이브 안에 무엇이 있나

세 포맷 모두 전체 형태는 비슷:

```
mypack.{zip,mrpack,gdlpack}/
├── <manifest>.json       ← 포맷별 매니페스트
├── overrides/            ← 번들된 설정, 스크립트, 선택적 스타터 월드
│   ├── config/
│   ├── scripts/
│   └── ...
└── modlist.html          ← (CurseForge 전용, 선택, 사람 읽기용 모드 목록)
```

매니페스트가 런처가 팩 설치에 필요한 유일한 파일. overrides 폴더는 결과 인스턴스에 풀릴 콘텐츠.

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

주요 필드:

- `minecraft.version`: 팩이 타깃하는 Minecraft 버전.
- `minecraft.modLoaders`: 어떤 로더의 어떤 버전 (포맷: `<loader>-<version>`).
- `files`: 각 모드는 CurseForge `projectID`와 `fileID`로 참조. GDLauncher가 CurseForge API로 해결해 다운로드.
- `overrides`: zip 안 폴더 이름. 모드 다운로드 완료 후 그 내용이 인스턴스로 복사됨.

`projectID`나 `fileID`의 모드가 CurseForge에서 삭제(작성자가 내림)되었다면 해당 모드에 대해 "file not found" 오류로 설치 실패.

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

CurseForge와의 주요 차이:

- `dependencies`로 Minecraft와 로더 버전을 직접 선언 (중첩 객체 없음).
- 각 `files` 엔트리에 **정확한 다운로드 URL**과 **해시**가 들어 있어 메타데이터 API 호출 없이 설치 가능.
- 각 파일이 client/server/양쪽 중 어디서 필수인지 `env`로 선언.

포맷이 단순하고 오프라인 설치가 쉬움 (URL이 박혀 있음). 더 엄격해서 해시 불일치는 경고가 아니라 강한 거부.

## GDLauncher: `gdlpack.json`

GDLauncher 자체의 팩 포맷은 **Export Instance**에서 **GDLauncher .gdlpack**을 골랐을 때 GDLauncher가 쓰는 포맷. 결정적 특징: 각 모드는 플랫폼별 ID나 URL이 아니라 콘텐츠 해시만으로 식별. GDLauncher가 설치 시점에 각 해시를 CurseForge와 Modrinth에 대해 해결하고 실제로 파일을 가진 쪽에서 다운로드.

### 왜 ID나 URL이 아니라 해시인가

- **하나의 소스로 크로스 플랫폼.** CurseForge와 Modrinth 둘 다 있는 모드는 양쪽에서 ID가 다름. `.gdlpack`은 신경 쓰지 않음. 해시 목록 하나로 양쪽 다 동작.
- **URL 부패 없음.** Modrinth CDN URL은 콘텐츠 주소 기반이라 안정적이지만, URL을 박으면 다운로드 출처가 고정됨. 해시는 한 플랫폼이 다운되어도 폴백 가능.
- **구조적 검증.** `mods/` 폴더로 쓰여지는 모든 바이트가 매니페스트 해시와 대조됨. 다른 JAR로 몰래 바꿔치기할 방법 없음.

### 아카이브 구조

```
mypack.gdlpack/
├── gdlpack.json          ← 매니페스트
├── .gdl/
│   └── icon.png          ← 임베디드 팩 아이콘 (선택)
└── overrides/            ← 번들된 파일 (설정, 스크립트, 해결 불가 모드)
    ├── config/
    └── ...
```

매니페스트는 루트에 위치. `.gdl/` 폴더는 런처가 임베드하는 메타데이터(현재는 아이콘만). overrides는 CurseForge와 동일하게 모드 해결 단계 후 결과 인스턴스에 풀립니다.

### 최소 매니페스트

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

로드에 필요한 건 `formatVersion`, `name`, `createdAt`, `dependencies.minecraft`, `entries`, `overrides`뿐.

### 전체 매니페스트 (주석 포함)

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

#### 최상위 필드

| 필드 | 타입 | 필수 | 의미 |
|---|---|---|---|
| `formatVersion` | integer | 예 | 매니페스트 스키마 버전. 현재 항상 `1`. |
| `name` | string | 예 | 팩 표시 이름. 임포트 시 기본 인스턴스 이름. |
| `version` | string | 아니오 | 팩 버전 (semver 권장, 예: `1.0.0`, `2.1.0-beta.1`). |
| `summary` | string | 아니오 | 한 줄 태그라인. |
| `author` | string | 아니오 | 팩 작성자나 팀. |
| `createdAt` | RFC 3339 타임스탬프 | 예 | 아카이브가 내보내진 시각. |
| `icon` | string | 아니오 | 아카이브 내 아이콘 파일 경로 (예: `.gdl/icon.png`). |
| `dependencies` | object | 예 | Minecraft와 모드 로더 요구사항 (아래 참조). |
| `entries` | array | 예 | 플랫폼 파일과 옵션 기능 (아래 참조). 순수 오버라이드 팩이라면 빈 배열도 유효. |
| `overrides` | string | 예 (기본값 `"overrides"`) | 인스턴스로 풀어 넣을 아카이브 내 디렉터리 이름. |
| `serverOverrides` | string | 아니오 | 서버 전용 오버라이드 디렉터리. |
| `clientOverrides` | string | 아니오 | 클라이언트 전용 오버라이드 디렉터리. |
| `source` | object | 아니오 | CurseForge나 Modrinth 팩에서 파생된 경우 원본을 가리킴 (아래 참조). |

#### `dependencies`

```json
{
  "minecraft": "1.20.1",
  "modloaders": [
    { "type": "forge", "version": "47.2.0", "primary": true }
  ]
}
```

- `minecraft`: 필요한 Minecraft 버전 (예: `1.20.1`, `1.21.4`, 스냅샷이라면 `25w20a`).
- `modloaders`: 0개 이상의 로더 엔트리. 각 엔트리:
  - `type`: `forge`, `neoforge`, `fabric`, `quilt` 중 하나 (소문자).
  - `version`: 팩이 빌드된 정확한 로더 버전.
  - `primary`: 여러 개가 있을 때 권장 로더를 표시 (예: Fabric과 Quilt 양쪽 호환 팩).

바닐라 팩은 `modloaders`가 빈 배열.

#### `entries`

`type` 필드로 구분되는 두 가지 엔트리 타입:

**`platform`**, 해시를 통해 해결되는 필수 모드:

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

세 해시 모두 필수. 해결 시점에 각각 다른 용도로 사용:

| 해시 | 사용처 |
|---|---|
| `sha512` | Modrinth 해결 (Modrinth의 `version_file` 엔드포인트), 다운로드 파일의 주요 무결성 검사. |
| `sha1` | Modrinth 해결 (폴백), 실행 시 Minecraft 자체가 파일 검증에 사용. |
| `murmur2` | CurseForge 해결 (CurseForge의 `fingerprints` 엔드포인트). |

설치 시점에 런처는 먼저 Modrinth (sha512 조회)를 시도하고, CurseForge (murmur2 핑거프린트)로 폴백하며, 어느 쪽에도 없으면 해당 엔트리는 실패. 해결된 파일은 항상 응답한 쪽 플랫폼에서 다운로드.

**`optional`**, 사용자가 포함하거나 제외할 수 있는, 스킵 가능한 모드 / 오버라이드 경로 그룹:

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

- `description`: 임포트 미리보기에서 사용자에게 표시되어 기능을 포함할지 결정하는 자료.
- `platforms`: 이 기능을 포함할 때만 다운로드되는 0개 이상의 해시 엔트리.
- `overridePaths`: 이 기능을 포함할 때만 풀리는 `overrides/` 안의 0개 이상의 경로(파일이나 폴더). 경로는 `overrides/`에 상대적.

기능은 플랫폼 파일만, 오버라이드만, 또는 둘 다 가질 수 있음. 관련된 선택적 콘텐츠를 묶는 데 사용: 셰이더 모드 + 설정, 설정뿐인 하드코어 난이도 프리셋, 선택적 리소스 팩 등.

#### `source`

기존 CurseForge나 Modrinth 모드팩 인스턴스에서 내보낸 경우, GDLauncher는 출처를 기록:

```json
{
  "platform": "curseforge",
  "projectId": 12345,
  "fileId": 67890,
  "name": "Original Pack",
  "url": null
}
```

또는

```json
{
  "platform": "modrinth",
  "projectId": "AANobbMI",
  "versionId": "abc123",
  "name": "Original Pack",
  "url": null
}
```

정보용으로, 런처는 이걸로 다운로드하지 않습니다 (그건 `entries` 배열의 역할). 공개 팩의 파생물이 원본 팩에 귀속되도록 존재.

### 번들 vs 매니페스트 전용 내보내기

인스턴스를 `.gdlpack`으로 내보낼 때 내보내기 마법사의 **Bundle Addons** 토글이 무엇이 어디로 들어갈지 결정합니다:

- **번들 OFF (매니페스트 전용).** GDLauncher가 CurseForge나 Modrinth로 해결할 수 있는 각 모드는 `entries`의 `platform` 엔트리가 됨. 실제 JAR은 아카이브에 *포함되지 않음*. 수신자의 런처가 임포트 시 플랫폼에서 재다운로드. GDLauncher가 해결할 수 없는 모드(수동으로 떨군 JAR 등)는 폴백으로 `overrides/mods/`에 번들됨. 결과: 작은 아카이브, 수신자는 설치 시 인터넷 필요.
- **번들 ON (자기 완결).** `platform` 엔트리는 출력되지 않고 모든 모드가 `overrides/mods/`로 복사. 결과: 큰 아카이브 (수백 MB~수 GB), Minecraft 에셋이 캐시되어 있으면 오프라인 설치 가능.

리소스 팩과 셰이더 팩도 같은 식으로 동작: 번들 OFF면 해결 가능한 건 `platform` 엔트리, 번들 ON이면 모두 직접 번들.

## 횡 비교: 세 포맷

| 속성 | CurseForge `.zip` | Modrinth `.mrpack` | GDLauncher `.gdlpack` |
|---|---|---|---|
| 매니페스트 파일명 | `manifest.json` | `modrinth.index.json` | `gdlpack.json` |
| 모드 식별 | CurseForge 프로젝트 + 파일 ID | 다운로드 URL + 해시 | 콘텐츠 해시만 |
| 해결 출처 | CurseForge API | 매니페스트 내 URL | CurseForge **또는** Modrinth (응답한 쪽) |
| 오프라인 설치 | 불가 (CDN 필요) | 가능 (URL 자체는 CDN 필요할 수 있음) | `Bundle Addons`가 켜져 있으면 가능 |
| 옵션 기능 | 모드별 `required: false` 플래그 | 파일별 `env` (client/server) | 여러 파일을 가질 수 있는 전용 `optional` 엔트리 |
| 오버라이드 폴더 | `overrides/` | 파일별 경로 | `overrides/` (설정 가능) |
| 서버 전용 파일 | 별도 팩 | `env.server` | `serverOverrides` 디렉터리 |
| 아이콘 | 외부 | 외부 | `.gdl/icon.<ext>`에 임베드 |
| 출처 추적 | 없음 | 없음 | `source` 필드 |
| 이식성 | 가장 넓음 (대부분 런처가 읽음) | Modrinth 지원 런처 | GDLauncher |

## 오버라이드 자세히

매니페스트가 오버라이드 폴더나 client-required로 표시된 팩 파일을 참조하면, 런처는 모드 해결 단계 후 그것들을 인스턴스에 풀어 넣습니다. 이렇게 팩이 번들할 수 있는 것들:

- 기본 모드 설정 (누가 설치해도 같은 초기 경험).
- KubeJS / CraftTweaker 스크립트.
- 스타터 월드 (가끔).
- 팩이 활성화 기대하는 리소스 팩.
- 팩 맞춤 `options.txt`.

오버라이드는 자동 생성 기본값을 이기지만 나중에 플레이어가 수동으로 바꾼 것에는 집니다. GDLPack은 추가로 `serverOverrides`와 `clientOverrides` 디렉터리를 지원해서, 한쪽에만 들어가야 하는 파일에 유용 (일치하는 클라이언트/서버 번들을 함께 배포하려는 팩용).

## 매니페스트가 오래됐을 때

팩 매니페스트는 팩 작성자가 마지막으로 게시한 스냅샷입니다. 매니페스트가 만들어진 뒤에 참조 모드가 소스 플랫폼에서 삭제되면, 작성자가 새 버전을 게시할 때까지 그 버전의 설치는 실패합니다. 옛 팩 버전은 깔끔하게 설치되는데 새 버전은 깨지는 게 이 경우: 새 버전이 지금은 사용할 수 없는 무언가를 참조합니다.

해결은 작성자 쪽 일이고, 런처는 매니페스트가 시키는 대로만 할 수 있습니다. GDLPack은 여기서 부분적 우위: 단일 플랫폼에 묶여 있지 않으므로, 모드가 CurseForge에서 사라져도 Modrinth에 남아 있으면(반대도 마찬가지) 같은 `entries` 배열이 여전히 해결됩니다.

## 매니페스트를 직접 읽기

특별한 도구 필요 없음. `mypack.mrpack`이나 `mypack.gdlpack`을 `mypack.zip`으로 이름 바꾸고, 아무 아카이브 도구로 열고, 안의 JSON 파일(`modrinth.index.json` 또는 `gdlpack.json`)을 보면 됩니다. CurseForge `.zip`의 `manifest.json`도 동일. 셋 다 평범한 JSON이라 어떤 텍스트 에디터로도 읽힘.

## 팩 만들기

공유용 팩을 만들 거라면 가장 쉬운 길은 GDLauncher에서 인스턴스를 구성한 다음 **Export Instance** (우클릭 → Export Instance)를 쓰는 것. 내보내기 마법사에서 대상 포맷을 고를 수 있음: CurseForge `.zip`, Modrinth `.mrpack`, 또는 GDLauncher 자체 `.gdlpack`. 수신자도 GDLauncher를 쓰고 옵션 기능이나 임베디드 아이콘이 필요하면 `.gdlpack`, 가장 넓은 크로스 런처 호환성을 원하면 `.zip`.
