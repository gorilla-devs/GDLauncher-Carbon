---
title: "Runtime Path와 App Data Path"
description: "GDLauncher가 사용하는 두 가지 경로. 각각 무엇을 담고 있는지, 왜 분리되어 있는지, 어느 쪽을 옮겨야 하는지."
faq:
  - question: "App Data Path와 Runtime Path의 차이는?"
    answer: "App Data Path는 Electron이 사용자별로 사용하는 표준 디렉터리로, 캐시와 runtime_path_override 마커가 들어갑니다. Runtime Path는 GDLauncher 코어가 무거운 것들(인스턴스, Minecraft 에셋과 라이브러리, Java 설치, 런처 데이터베이스, 앱 전역 로그)을 저장하는 곳입니다. 기본적으로 Runtime Path는 App Data Path 안에 있지만, 옮길 수 있는 건 Runtime Path 쪽입니다."
  - question: "App Data Path는 어디에 있나요?"
    answer: "Windows: C:\\Users\\<사용자>\\AppData\\Roaming\\gdlauncher_carbon. macOS: /Users/<사용자>/Library/Application Support/gdlauncher_carbon. Linux: $XDG_DATA_HOME/gdlauncher_carbon, 미설정이면 ~/.local/share/gdlauncher_carbon."
  - question: "어느 쪽을 옮겨야 하나요?"
    answer: "Runtime Path입니다. 인스턴스를 추가할 때마다 커지는 쪽이고, App Data Path는 작게 유지되며 OS 사용자 프로필에 묶입니다. GDLauncher는 Settings → Runtime Path에서 Runtime Path 이동을 제공하지만 App Data Path는 Electron이 관리하므로 UI에서 옮길 수 없습니다."
  - question: "runtime_path_override 파일은 무엇인가요?"
    answer: "Runtime Path를 바꾸면 GDLauncher가 App Data Path 안에 runtime_path_override라는 작은 텍스트 파일을 만듭니다. 내용은 새 Runtime Path 문자열이고, 매번 실행할 때 이를 읽어 데이터 위치를 알아냅니다. 지우면 기본 Runtime Path로 폴백됩니다."
  - question: "두 대의 컴퓨터가 Runtime Path를 공유할 수 있나요?"
    answer: "안 됩니다. 데이터베이스에 PC별 상태(경로, 토큰, Java 설치)가 저장돼 있고 동시 사용을 가정하지 않습니다. 다른 PC에서 같은 인스턴스를 쓰고 싶다면 수동 복사 또는 Cloud Instance Share 기능을 사용하세요."
---

# Runtime Path와 App Data Path

## 두 개의 경로, 두 가지 역할

GDLauncher는 파일을 두 곳에 나누어 둡니다. Electron 쪽 작은 것들을 위한 **App Data Path**, 무거운 데이터(인스턴스, 에셋, Java, 데이터베이스)를 위한 **Runtime Path**. 가끔 옮기고 싶어지는 건 Runtime Path이고, App Data Path는 거의 손댈 일이 없습니다.

### App Data Path

OS 표준 사용자 앱 디렉터리, 즉 Electron의 `userData`가 가리키는 위치입니다. GDLauncher는 여기에 다음을 둡니다:

- `runtime_path_override` 마커 (Runtime Path가 실제로 어디에 있는지 알려주는 한 줄짜리 텍스트 파일)
- 옮기지 않았다면 기본 Runtime Path가 들어가는 `data/` 하위 폴더
- Electron 자체의 Chromium 캐시 (Network/, GPUCache/, Cookies 등)
- Electron 메인 프로세스 로그

OS 표준 위치:

- **Windows:** `C:\Users\<사용자>\AppData\Roaming\gdlauncher_carbon`
- **macOS:** `/Users/<사용자>/Library/Application Support/gdlauncher_carbon`
- **Linux:** `$XDG_DATA_HOME/gdlauncher_carbon`, 미설정이면 `~/.local/share/gdlauncher_carbon`

`data/` 하위 폴더를 빼면 보통은 작은 디렉터리입니다. GDLauncher에는 이걸 옮기는 설정이 없고, OS 규약과 Electron이 현재 위치를 전제로 합니다.

### Runtime Path (Core Module)

GDLauncher의 Rust 코어가 그 외의 모든 걸 놓는 곳:

- 인스턴스(`instances/` 하위)
- Minecraft 공유 에셋(Mojang이 제공하는 텍스처, 사운드)
- Minecraft 공유 라이브러리(Mojang과 모드 로더의 JAR)
- GDLauncher가 다운로드한 Java
- 런처 데이터베이스 `gdl_conf.db`
- 앱 전역 로그 (`__gdl_logs__/`)

기본 위치는 `<App Data Path>/data/`라서 App Data 안에 있습니다. **Settings → Runtime Path**에서 원하는 곳으로 바꿀 수 있습니다. 큰 모드팩을 설치하면 금세 50 GB를 넘기는 경로입니다.

## Runtime Path를 옮기는 경우

흔한 이유 두 가지:

1. **SSD가 가득 참.** 인스턴스를 큰 HDD나 보조 SSD로 옮기고 싶을 때.
2. **OS 사용자 프로필과 분리해서 관리하고 싶음.** 백업을 따로 가는 경우. 단, 플레이 중에 동기화 도구가 돌면 파일 락 충돌이 생기므로 피하세요.

평상시 용도라면 Runtime Path를 굳이 옮기지 않아도 됩니다. 기본 위치가 적합합니다.

## 이동 방법

**Settings → Runtime Path**를 열고 새 위치를 입력하거나 폴더 아이콘으로 선택합니다. 현재 경로와 다르고 유효한 위치일 때 행 오른쪽의 원형 화살표 적용 버튼이 켜집니다. 누르면 옛 경로와 새 경로를 보여주는 확인 모달이 열립니다.

대상 폴더가 비어 있거나 없으면, 확인 시 본격 마이그레이션이 진행됩니다. 오버레이가 스캔, 파일별 복사, 원본 파일별 삭제 순으로 표시됩니다. 도중에 앱이나 컴퓨터를 종료하지 마세요. 끝나면 런처가 자동으로 재시작합니다.

대상에 이미 GDLauncher Runtime Path 데이터가 있으면(전에 옮긴 적이 있어 새 설치가 그 데이터를 가리키도록 하고 싶은 경우), 모달이 노란색으로 "비어 있지 않음"을 알립니다. 확인을 누르면 마커만 기존 데이터를 가리키도록 다시 쓰이고, 파일 복사는 일어나지 않고 런처가 재시작합니다. 옛 위치의 데이터는 고아 상태로 남으니 직접 지우면 됩니다.

도중에 실패하면 오버레이가 빨갛게 바뀌고 에러가 표시됩니다. 런처가 롤백해서 새 경로에 만든 파일을 지우고 마커는 옛 경로를 가리킨 채로 남기므로, 데이터 손실 없이 다시 시도할 수 있습니다. 흔한 두 원인은 대상 드라이브의 쓰기 권한 부족과 디스크 공간 부족입니다.

### runtime_path_override 마커

Runtime Path를 바꾸면 GDLauncher가 **App Data Path** 안에(Runtime Path 안이 아니라) `runtime_path_override`라는 텍스트 파일을 씁니다. 내용은 새 Runtime Path의 일반 텍스트입니다. 매번 실행할 때 이 파일을 읽어 데이터 위치를 파악합니다.

실수로 지웠다면 GDLauncher가 기본 Runtime Path(`<App Data>/data/`)로 폴백합니다. 데이터 자체는 옮긴 곳에 그대로 있으므로 사라지지 않지만, 런처가 인식하지 못합니다. **Settings → Runtime Path**에서 옮긴 폴더를 다시 가리키면 복구됩니다. 폴더에 이미 GDLauncher 데이터가 있으므로 런처는 "switch only" 작업으로 처리해 파일 복사 없이 마커만 갱신합니다.

## 데이터베이스를 공유하면 안 되는 이유

Runtime Path의 `gdl_conf.db` 파일에는 계정 토큰, Microsoft 리프레시 토큰, GDL 계정 상태, 인스턴스 메타데이터가 담깁니다. 머신 로컬이며 민감한 자격 증명이 포함되므로 **누구에게도 보내지 마세요.** 두 대의 PC가 같은 DB를 쓰면 토큰 리프레시 경쟁으로 양쪽 모두 로그아웃됩니다.

다른 PC에서 같은 인스턴스를 쓰려면 `instances/` 안의 인스턴스 폴더를 수동으로 복사하거나, 그 용도로 만든 Cloud Instance Share 기능을 쓰세요.
