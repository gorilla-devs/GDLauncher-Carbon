---
title: "문제 해결"
description: "GDLauncher와 Minecraft의 일반적인 실행 문제를 해결합니다. 앱 데이터 경로, 런타임 경로, 로그 위치 및 검증된 해결책을 안내합니다."
faq:
  - question: "GDLauncher는 데이터를 어디에 저장하나요?"
    answer: "Windows: C:\\Users\\<사용자>\\AppData\\Roaming\\gdlauncher_carbon. macOS: /Users/<사용자>/Library/Application Support/gdlauncher_carbon. Linux: $XDG_DATA_HOME/gdlauncher_carbon (XDG가 설정되어 있지 않다면 ~/.local/share/gdlauncher_carbon)."
  - question: "GDLauncher 로그는 어디에 있나요?"
    answer: "GDLauncher는 앱 레벨 로그를 두 개의 파일에 기록합니다: 앱 데이터 폴더의 main.log (Electron)와 런타임 경로 내 __gdl_logs__ 폴더의 타임스탬프 포함 <timestamp>.log (Rust 코어, 최신 10개 유지). 문제를 보고할 때는 둘 다 보내주세요. 정확한 경로는 Share App Logs 가이드를 참고하세요."
  - question: "GDLauncher가 열리지 않습니다. 어떻게 해야 하나요?"
    answer: "먼저 데이터 폴더의 로그에서 오류를 확인하세요. 흔한 원인은 런타임 손상, 백신이 실행 파일을 차단, 또는 부분적으로만 적용된 업데이트입니다. GDLauncher를 깨끗이 재설치하고 인스턴스를 복원하면 두 경우 모두 해결되는 경우가 많습니다."
  - question: "모드팩이 시작 시 충돌하는 이유는 무엇인가요?"
    answer: "시작 시 충돌의 대부분은 Minecraft 버전, 모드 로더, 모드 간 호환성 문제로 발생합니다. __gdl_logs__ 폴더에서 가장 최신 로그 파일을 열어 오류를 확인하세요. 특정 모드 이름이 표시된다면 보통 그것이 원인입니다. Addons 탭에서 비활성화하고 다시 실행하세요. OutOfMemoryError라면 인스턴스 설정에서 RAM을 늘리세요."
  - question: "GDLauncher를 다른 드라이브나 폴더로 옮기려면 어떻게 하나요?"
    answer: "설정 → 일반 → 런타임 경로를 엽니다. 새 위치로 변경하면 GDLauncher가 인스턴스와 다운로드를 자동으로 마이그레이션합니다. 마이그레이션은 다음 실행 시 한 번 수행됩니다."
  - question: "GDLauncher를 오프라인에서 사용할 수 있나요?"
    answer: "이미 설치된 인스턴스는 오프라인에서 플레이할 수 있습니다. 다만 인증은 최초 1회 온라인 연결이 필요하며(Microsoft 계정), 새 모드나 모드팩을 다운로드하려면 인터넷 연결이 필요합니다."
---

## 앱 데이터 경로

GDLauncher가 Electron의 데이터와 기본적으로 Core Module 런타임 경로를 저장하는 경로입니다.

### Windows

`C:\Users\\{{사용자 이름}}\\AppData\Roaming\gdlauncher_carbon`

### macOS

`/Users/{{사용자 이름}}/Library/Application Support/gdlauncher_carbon`

### Linux

- `$XDG_DATA_HOME` 환경 변수가 설정된 경우: `$XDG_DATA_HOME/gdlauncher_carbon`
- 설정되지 않은 경우: `{{homedir}}/.local/share/gdlauncher_carbon`

[homedir 자세한 내용](https://nodejs.org/api/os.html#oshomedir)

## Core Module 런타임 경로

Core Module이 모든 데이터(인스턴스, 에셋, 라이브러리 등)를 저장하는 경로입니다.
일반적으로 앱 데이터 경로의 `data` 하위 폴더에 위치하며, 다른 위치를 직접 지정한 경우는 예외입니다.

### 앱 데이터베이스

앱 데이터베이스는 Core Module 런타임 경로에 위치하며, `gdl_conf.db`라는 SQLite 데이터베이스 파일입니다.

**이 파일에는 민감한 데이터가 포함되어 있으므로 누구에게도 보내지 마세요.**

### 앱 로그

GDLauncher는 앱 레벨 로그를 두 개의 파일에 기록합니다. 지원 요청에는 **항상 둘 다** 보내세요. 런처의 두 프로세스는 서로 작업을 주고받기 때문에 한쪽의 실패 원인이 다른 쪽 로그에 나타나는 경우가 많습니다.

- **`main.log`** (App Data Path 내): Electron 메인 프로세스 로그. 창 생성, IPC, 자동 업데이트, 네이티브 다이얼로그, 데스크톱 셸의 하드 크래시 등을 기록.
- **`__gdl_logs__/<timestamp>.log`** (Core Module Runtime Path 내): Rust 코어 로그. 계정 로그인, 에셋 다운로드, 모드 로더 설치, 인스턴스 실행, 설정 변경 등을 기록. 최신 10개 유지.

OS별 경로와 스크린샷은 [Share App Logs](/guides/share-app-logs)를 참고.

**로그에는 민감한 데이터가 포함될 수 있으므로 공유 시 주의하세요.**

### 런타임 경로 변경

런타임 경로를 변경하면, 앱이 모든 인스턴스와 설정 파일을 새 위치로 자동으로 이동시킵니다.

대상 폴더가 이미 사용 중이라면, 앱은 런타임 경로 설정만 전환하고 어떤 파일도 이동하거나 복사하지 않습니다.

#### 마이그레이션 오류

마이그레이션이 실패하면 앱이 오류 메시지를 표시합니다.

먼저 메시지가 무엇을 의미하는지 이해해 보세요.
모든 파일이 정상적으로 복사되었다면, 보통 이전 파일을 삭제하는 단계에서 오류가 발생한 것입니다. 앱을 닫고 이전 파일을 수동으로 삭제할 수 있습니다.

이전 런타임 경로의 `runtime_path_override` 파일은 삭제하지 마세요. 이 파일은 앱이 런타임 경로 변경 여부를 감지하는 데 사용됩니다.

확신이 서지 않으면 [Discord 서버](https://discord.gdlauncher.com)에 참여하여 도움을 요청하세요.
