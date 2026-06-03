---
title: "인스턴스 폴더의 구조"
description: "GDLauncher 인스턴스 폴더 안에 무엇이 있는지. 모드, 월드, 설정, 스크린샷, 로그가 어디 있고 손으로 지워도 안전한 것은 무엇인지."
faq:
  - question: "Minecraft 월드는 어디에 저장되나요?"
    answer: "각 월드는 <runtime_path>/instances/<instance>/instance/saves/<world-name>/에 있습니다. saves/ 폴더는 처음 월드를 생성하거나 가져왔을 때 만들어집니다. 인스턴스를 오른쪽 클릭 → Open Folder로 폴더를 열고 instance/saves로 들어가세요. 백업은 월드 폴더를 통째로 복사."
  - question: "크래시 리포트는 어디에 있나요?"
    answer: "인스턴스 내 instance/crash-reports/. 폴더는 Minecraft가 실제로 크래시했을 때만 존재하며, 게임이 심하게 죽은 적이 없다면 아직 없습니다. 각 리포트는 타임스탬프가 있는 텍스트 파일(crash-<date>-server.txt 등)."
  - question: "모드 JAR은 어디에 두나요?"
    answer: "인스턴스 내 instance/mods/. 손으로 JAR을 넣어도 작동하지만 GDLauncher 추적 밖이 됩니다. 특별한 이유가 없으면 Addons 탭을 쓰세요. 잠긴 모드팩 인스턴스는 Addons의 Add가 비활성화되지만 수동 복사는 통과합니다."
  - question: "런처 로그와 Minecraft 로그의 차이는?"
    answer: "두 종류. 런처 쪽 세션 로그는 <instance>/logs/. Minecraft 자체 게임 로그(latest.log, crash-reports 등)는 한 단계 더 들어간 <instance>/instance/logs/와 <instance>/instance/crash-reports/."
  - question: "instance.json과 packinfo.json은 무엇인가요?"
    answer: "GDLauncher가 모든 인스턴스의 최상위에 쓰는 메타데이터 파일. instance.json은 인스턴스 이름, 아이콘, 모드 로더, Minecraft 버전, 마지막 플레이 시간, 누적 플레이 시간을 담습니다. packinfo.json은 CurseForge/Modrinth 모드팩과 페어된 인스턴스에만 존재하며, 팩에 속한 파일을 추적해 팩 관리 모드와 사용자가 추가한 모드를 구분합니다. 둘 다 손으로 지우지 마세요."
---

# 인스턴스 폴더의 구조

## 인스턴스 폴더의 위치

각 GDLauncher 인스턴스는 Runtime Path의 하위 폴더에 있습니다:

```
<runtime_path>/
└── instances/
    └── <shortpath>/        ← 인스턴스 폴더
```

`<shortpath>`는 표시명을 정규화한 형태. GDLauncher에서 인스턴스를 오른쪽 클릭 → **Open Folder**로 바로 갈 수 있습니다.

## 내부 구조

인스턴스 폴더는 최상위에 GDLauncher가 추적하는 몇 가지와, Minecraft의 실제 게임 디렉터리인 `instance/` 하위 폴더로 나뉩니다. 항상 있는 것도 있고, 무언가가 처음 쓸 때 만들어지는 것도 많습니다.

```
<shortpath>/
├── instance.json          ← GDLauncher의 인스턴스 메타데이터 (항상 존재)
├── packinfo.json          ← 모드팩 페어링 정보 (페어된 모드팩 한정)
├── icon.png | icon.webp   ← 커스텀 아이콘 (설정 시에만)
├── logs/                  ← GDLauncher의 인스턴스별 런처 로그
└── instance/              ← Minecraft의 게임 디렉터리
    ├── mods/              ← 모드 JAR
    ├── config/            ← 모드 설정
    ├── shaderpacks/       ← 셰이더 팩 (설치 시)
    ├── options.txt        ← Minecraft 클라이언트 설정 (첫 실행 후)
    ├── logs/              ← Minecraft 세션 로그 (latest.log 등)
    ├── saves/             ← 월드 (생성 시 만들어짐)
    ├── screenshots/       ← F2 스크린샷 (첫 F2 시 만들어짐)
    ├── crash-reports/     ← 크래시 덤프 (게임이 크래시했을 때만)
    ├── resourcepacks/     ← 커스텀 리소스 팩 (추가 시)
    ├── datapacks/         ← 글로벌 데이터 팩 (월드 단위는 saves/<world>/datapacks/)
    └── (팩 전용)           ← kubejs/, defaultconfigs/, packmenu/ 등 팩이 제공할 때만
```

새 인스턴스에서 이 중 일부가 없어도 놀라지 마세요. 런처와 Minecraft는 필요할 때 필요한 것만 만듭니다. 한 번도 플레이하지 않은 인스턴스에는 `saves/`, `screenshots/`, `options.txt`가 없습니다. 바닐라 인스턴스에는 `mods/`도 `config/`도 없습니다.

## 각 항목의 내용

### 최상위 파일

- **`instance.json`**: GDLauncher 메타데이터, 이름, 아이콘 경로, 모드 로더와 버전, Minecraft 버전, 생성 시각, 마지막 플레이 시각, 누적 플레이 시간. 항상 존재.
- **`packinfo.json`**: 모드팩에서 온 파일들의 해시 매니페스트. 런처가 팩 관리 모드와 사용자가 추가한 것을 구분할 수 있게 합니다. CurseForge/Modrinth 모드팩과 페어된 인스턴스에만 존재.
- **`icon.png`** 또는 **`icon.webp`**: 업로드한 커스텀 아이콘. 기본 아이콘이라면 없습니다.

### `logs/` (최상위)

GDLauncher 자체의 인스턴스별 로그. 오른쪽 클릭 메뉴의 **View Logs** 액션이 보여주는 곳. *런처가 본* 실행(자바 인자, 에셋 다운로드, 모드 로더 설치, 종료 코드)을 다루며, 게임이 자체 로그를 쓰기 전에 죽었을 때 특히 유용합니다.

### `instance/mods/`

모드 JAR 파일들. Minecraft가 시작 시 여기 있는 것을 모두 로드합니다(모드 로더 규칙에 따름). 런처는 어떤 모드가 인스턴스에 속하는지 자체 DB에서 추적합니다(파일명과 콘텐츠 해시를 키로 사용), 사이드카 파일은 없습니다. 수동으로 넣은 JAR도 인식되지만 CurseForge/Modrinth 메타데이터는 런처에 없습니다.

### `instance/config/`

모드당 하나의 하위 폴더 또는 파일. 모드 설정의 영속 저장소. 대부분의 모드는 `config/<modid>.toml` 또는 `config/<modid>/`를 작성합니다. 수동 편집은 대체로 안전하고 게임 재시작 시 적용됩니다.

### `instance/resourcepacks/`, `instance/shaderpacks/`, `instance/datapacks/`

에셋 팩. 리소스 팩은 텍스처와 사운드, 셰이더 팩은 렌더링(Iris/OptiFine 등을 모드로 설치한 상태), 데이터 팩은 레시피/루트/함수 추가. 특정 월드용 데이터 팩은 `saves/<world>/datapacks/`에 들어갑니다. 이 폴더들은 실제로 콘텐츠가 있을 때만 생성됩니다.

### `instance/saves/`

월드마다 하위 폴더 하나. 내부의 `level.dat`이 월드 마스터, `region/`에 청크 데이터, `playerdata/`에 플레이어별 상태, `datapacks/`에 월드 스코프 데이터 팩. 월드 백업은 `<world>/` 폴더 전체 복사. `saves/` 자체는 처음 월드를 생성할 때 만들어집니다.

### `instance/screenshots/`

게임 내 F2로 찍은 PNG, 타임스탬프로 이름이 매겨짐. 첫 스크린샷 시 만들어집니다.

### `instance/logs/`와 `instance/crash-reports/`

Minecraft 자체의 진단 출력. `logs/latest.log`는 항상 가장 최근 실행(다음 실행 시 `logs/<date>-1.log.gz`로 롤됨). `crash-reports/`는 완전한 크래시 덤프이며, 실제로 크래시가 발생했을 때만 생성됩니다.

### `instance/options.txt`

Minecraft 클라이언트 설정(그래픽, 컨트롤, 사운드). 평범한 텍스트, key=value. 정말 원하면 수동 편집 가능.

### 모드팩 전용 폴더

대형 모드팩은 추가 폴더를 함께 제공합니다. 가장 흔한 것:

- **`kubejs/`**: KubeJS 스크립트(`server_scripts/`, `client_scripts/`, `startup_scripts/`, `data/`, `assets/`). 팩 저자가 런타임 조정에 사용.
- **`defaultconfigs/`**: "기본 설정이 어떻게 보여야 하는지"의 스냅샷. 팩 실행 스크립트가 매 시작마다 누락 항목을 `config/`에 복사.
- **`packmenu/`**: 팩 테마 메인 메뉴 에셋(커스텀 버튼, 배경, 스플래시 텍스트).
- **`defaultsettings/`**: `defaultconfigs/`와 비슷하지만 `options.txt`와 키바인딩용.

팩이 제공할 때만 존재. 바닐라와 대부분의 커스텀 인스턴스에는 없습니다.

## 삭제해도 안전한 것

| 폴더 | 안전? | 효과 |
|---|---|---|
| `instance/mods/`의 특정 JAR | 예 | 해당 모드 사라짐. 그 모드를 쓰던 월드는 깨질 수 있음. |
| `instance/config/<modid>/` | 예 | 다음 실행 시 모드가 기본값으로 리셋. |
| `instance/resourcepacks/`, `instance/shaderpacks/` 내용 | 예 | 팩 사라짐. |
| `instance/saves/<world>/` | 예 | 월드 영구 삭제. 먼저 백업하세요. |
| `instance/logs/`, `crash-reports/` | 예 | 디스크 공간 회수. |
| `instance/screenshots/` | 예 | 옛 스크린샷 안녕. |
| `logs/` (런처 로그) | 예 | 위와 동일. |
| `instance/options.txt` | 예 | 게임 설정이 기본값으로. |
| `instance.json` | 안 됨 | 런처가 인스턴스를 추적하지 못함. |
| `packinfo.json` | 가능 (사실상 페어 해제) | 런처가 페어된 모드팩으로 다루지 않게 됨. 인스턴스 Settings 탭의 Unpair 버튼과 같은 효과지만 지저분. |
| `instance/` 폴더 전체 | 안 됨 | 인스턴스가 망가집니다. 런처 UI의 Delete를 쓰세요. |

## 잠긴 모드팩 인스턴스에 관한 메모

인스턴스 폴더는 디스크상의 보통 폴더입니다. GDLauncher가 모드팩 인스턴스에 거는 잠금은 UI 레벨에서 강제되며 파일시스템에서 강제되지 않습니다. 잠긴 인스턴스의 `instance/mods/`에 JAR을 직접 떨어뜨리면 기계적으로 작동하지만 런처는 Addons 탭으로 인식하지 못합니다. 그런 파일을 제거하려면 파일시스템으로 돌아가야 합니다. 더 안전한 작업 흐름을 원한다면 먼저 인스턴스를 열고 **Settings** 탭의 Modpack 섹션에서 **Unlock**을 누르세요.
