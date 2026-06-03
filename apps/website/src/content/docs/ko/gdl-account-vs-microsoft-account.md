---
title: "GDL 계정 vs Microsoft 계정"
description: "GDLauncher가 다루는 두 가지 계정. Minecraft 플레이용 Microsoft, 공유와 소셜 기능을 위한 GDL. 각각의 역할과 필요성."
faq:
  - question: "Minecraft를 플레이하려면 GDL 계정이 필요한가요?"
    answer: "아니요. 플레이에는 Minecraft Java Edition을 구매한 Microsoft 계정만 필요합니다. GDL 계정은 선택 사항이며, GDLauncher 자체 기능(인스턴스 공유, 친구 코드, 표시명 기록, 프로필 편집)만을 해제합니다. 없어도 GDLauncher는 잘 작동합니다."
  - question: "GDL 계정으로 무엇을 할 수 있나요?"
    answer: "지금은 주로 인스턴스 공유: 오른쪽 클릭 → Share로 코드를 만들고 다른 GDLauncher 사용자가 붙여넣어 가져옵니다. 변경 이력이 있는 안정적인 표시명, 공유 미리보기에서 본인을 식별해주는 친구 코드가 있는 프로필 카드도 함께 제공됩니다. 다른 GDLauncher 사용자와의 모든 상호작용은 GDL 계정을 거칩니다."
  - question: "Microsoft 계정 없이 GDLauncher를 쓸 수 있나요?"
    answer: "안 됩니다. Microsoft 계정이 Minecraft 소유를 증명하고 Mojang에서 실행 토큰을 받습니다. 그게 없으면 Minecraft 서버 쪽에 인증할 수단이 없습니다."
  - question: "Microsoft 계정을 여러 개 가질 수 있나요?"
    answer: "가능합니다. Settings → Accounts에 표로 표시되어 추가, 삭제, 활성 전환을 할 수 있습니다. 활성 계정(Play가 쓰는 것)은 가장 왼쪽 열에서 강조됩니다."
  - question: "GDL 프로필의 친구 코드는 뭔가요?"
    answer: "GDL 계정에 부여된 짧고 안정적인 식별자입니다. 표시명을 바꿔도 변하지 않고, 공유 미리보기에 표시되어 다른 사용자가 누가 공유했는지 알 수 있게 합니다. Settings → Accounts → GDL Account 프로필 카드에서 복사하세요."
---

# GDL 계정 vs Microsoft 계정

## 두 계정 체계, 하나의 런처

GDLauncher에는 두 개의 계정 체계가 있습니다. **Microsoft**는 Minecraft 소유를 증명하는 쪽이고 플레이에 필수입니다. **GDL**은 GDLauncher 자체의 선택 계정으로, GDL 백엔드를 사용하는 기능(인스턴스 공유, 프로필, 표시명 기록)에 쓰입니다.

### Microsoft 계정

Minecraft Java Edition을 구매한 계정, 게임 라이선스를 보유한 쪽입니다. Microsoft는 이를 실행에 요구하고, GDLauncher가 Microsoft에 사인인해 토큰을 받아 실행 시 Mojang에 전달하여 서버에 소유를 증명합니다.

플레이하려면 최소 한 개의 사인인된 Microsoft 계정이 필요합니다. 없으면 Play 버튼은 아무 일도 하지 않습니다.

계정별 로컬 저장: 액세스 토큰, 리프레시 토큰, ID 토큰, Minecraft 사용자명과 UUID, 스킨 참조, 액세스 토큰 만료. 런처가 리프레시 토큰으로 백그라운드에서 액세스 토큰을 갱신하므로 보통은 모르게 지나갑니다.

해제하는 기능: Minecraft 실행, 서버 참가, 게임 소유.

### GDL 계정

GDLauncher 자체 계정 체계. 선택. GDLauncher가 직접 제공하는 기능(Microsoft가 신경 쓸 일이 아닌 것들)을 위해서만 존재합니다.

이메일과 표시명으로 가입하면 안정적인 친구 코드가 발급됩니다. 거기서부터 다른 GDLauncher 사용자가 관여하는 기능들을 사용할 수 있습니다.

로컬에는 연결 정보만 저장: 이 GDL 신원이 속한 Microsoft 계정과 GDL 백엔드와 통신하기 위한 JWT. 표시명, 친구 코드, 이메일, 프로필 사진 등은 GDL 백엔드에 있고 UI가 필요할 때 가져옵니다.

해제하는 기능:

- **인스턴스 공유.** 오른쪽 클릭 → Share로 코드 생성, 다른 GDLauncher 사용자가 붙여넣어 가져오기.
- **표시명 기록.** 표시명을 바꾸면 변경 기록이 추적됩니다. 프로필 카드에서 과거 이름을 보고 원하면 지울 수 있습니다.
- **프로필 편집.** 표시명, 프로필 사진, 복구 이메일 설정 모두 Settings → Accounts의 GDL 프로필 카드에서.

## 언제 어느 쪽이 필요한가

| 시나리오 | Microsoft | GDL |
|---|---|---|
| Minecraft 실행만 | 필수 | 불필요 |
| CurseForge/Modrinth에서 모드와 모드팩 설치 | 필수 | 불필요 |
| 친구에게 인스턴스 공유 | 필수 | 필수 |
| 공유 코드 받기 | 필수 | 필수 |
| 친구 시스템 사용 | 필수 | 필수 |
| 오프라인(이미 설치한 인스턴스) | 캐시 인증으로 잠시 작동 | 불필요 |

## 관리 방법

둘 다 **Settings → Accounts**에 있습니다.

GDL Account 섹션은 페이지 상단. 사인아웃 상태면 Sign in / Sign up 버튼. 사인인 후엔 표시명, 친구 코드(복사 가능), 복구 이메일, 인증 상태가 담긴 프로필 카드. 하단 Danger Zone에서 7일 쿨다운으로 계정 삭제 예약 가능.

Microsoft Accounts 섹션은 아래에 표로. 열: Active, Username, Type, Status, UUID, Actions. Status는 각 계정 토큰의 상태를 표시합니다:

- **ok**(녹색 체크): 토큰 유효, 실행 가능.
- **expired**(노란색 경고): 토큰 만료. Actions 열에 리프레시 아이콘이 나타나며, 클릭하면 Microsoft 사인인 흐름으로 다시 보냅니다.
- **refreshing**(노란색 리프레시): 백그라운드에서 갱신 중. 별도 작업 불필요.
- **invalid**(빨간색 X): 갱신 실패. expired와 같은 리프레시 아이콘, 클릭하면 Microsoft 사인인 흐름으로 안내합니다.

활성 계정을 바꾸려면 원하는 행의 Active 셀을 클릭. 활성 행에는 더블 체크 아이콘이 표시되고, 다른 행은 호버 시 흐릿하게 보입니다.

## 계정 삭제

유일한 Microsoft 계정을 삭제하면 GDLauncher에서 완전히 사인아웃되어 홈으로 이동합니다.

GDL 계정에 연결된 Microsoft 계정을 삭제하려 하면 삭제 전에 정말 연결을 끊을지 확인하는 모달이 뜹니다.

GDL 계정 삭제는 7일 지연 작업. 쿨다운 동안 같은 페이지에서 취소할 수 있습니다.
