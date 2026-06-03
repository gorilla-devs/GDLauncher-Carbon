---
title: "Microsoft 인증 오류"
description: "GDLauncher의 일반적인 Microsoft 인증 오류를 해결하세요. Invalid Grant, 계정 차단, 콘솔 액세스 필요, Xbox Live 오류 등에 대한 해결책을 안내합니다."
faq:
  - question: "GDLauncher에서 'Invalid Grant' 오류가 나는 이유는 무엇인가요?"
    answer: "'Invalid Grant' 오류는 보통 Microsoft 계정의 보안과 관련된 문제일 가능성이 큽니다. 가장 흔한 해결책은 Microsoft 계정에서 2단계 인증을 활성화하거나, 비밀번호가 없으면 비밀번호를 설정하거나, 로그아웃 후 다시 로그인하는 것입니다."
  - question: "GDLauncher가 제 계정이 차단되었다고 표시하는 이유는 무엇인가요?"
    answer: "GDLauncher가 계정을 차단된 것으로 표시한다면, 그 차단은 Mojang 또는 Microsoft에서 발생한 것이며 GDLauncher 측에서 차단한 것이 아닙니다. minecraft.net 또는 Microsoft 계정에 로그인하여 차단 사유를 확인하세요. GDLauncher는 인증 응답을 그대로 전달할 뿐, 자체 차단 목록은 없습니다."
  - question: "GDLauncher가 콘솔 액세스가 필요하다고 하는 이유는 무엇인가요?"
    answer: "이는 일반적으로 자녀 계정이거나 가족 그룹 제한이 적용된 계정에서 나타납니다. 부모 계정이 자녀 계정에게 해당 플랫폼에서 Minecraft를 플레이할 수 있는 권한을 부여해야 합니다. account.microsoft.com/family에서 가족 설정을 조정하세요."
  - question: "Xbox Live 인증 오류가 계속 나타납니다. 어떻게 해야 하나요?"
    answer: "Xbox Live 오류는 일반적으로 Microsoft 계정의 국가/지역 설정이 Xbox Live를 허용하지 않거나, 계정이 Xbox Live 이용 약관에 동의하지 않은 경우에 발생합니다. 동일한 Microsoft 계정으로 xbox.com에 한 번 로그인하여 약관에 동의한 후 GDLauncher에서 다시 시도하세요."
  - question: "GDLauncher를 사용하려면 Minecraft를 다시 구매해야 하나요?"
    answer: "아니요. GDLauncher는 기존 Microsoft / Mojang Minecraft 계정을 그대로 사용합니다. 별도의 구매나 구독이 필요하지 않습니다. Minecraft Java Edition을 이미 보유하고 있다면 같은 계정으로 GDLauncher에 로그인할 수 있습니다."
---

# Microsoft 인증 오류

GDLauncher에서 Microsoft 계정으로 로그인할 때, 런처가 당신을 대신해 Microsoft OAuth 서비스와 Mojang 인증 API와 통신합니다. 그 서비스들에서 발생한 오류는 런처에 그대로 표시되며, 문구는 GDLauncher가 아니라 Microsoft에서 옵니다.

다음은 가장 흔한 오류와 그 의미입니다.

## Invalid Grant

Microsoft가 OAuth 교환을 거부할 때 나타납니다. 일반적인 원인:

- 계정에 비밀번호가 설정되어 있지 않음 (이메일 링크나 소셜 로그인으로 만든 Microsoft 계정). [account.microsoft.com](https://account.microsoft.com)에서 비밀번호를 추가하세요.
- 계정이 2단계 인증 없는 옛 로그인 흐름을 사용 중. [account.microsoft.com/security](https://account.microsoft.com/security)에서 2FA를 켜면 대부분 해결됩니다.
- 캐시된 토큰이 오래됨. **Settings → Accounts**에서 계정을 로그아웃했다가 다시 로그인하세요.

## 계정 차단

GDLauncher는 Mojang의 응답을 그대로 전달합니다. 차단은 Mojang 쪽에서 일어난 것이며, GDLauncher는 자체 차단 목록을 운영하지 않습니다. 차단 사유와 이의신청 방법은 동일 계정으로 [minecraft.net](https://minecraft.net)에 로그인해 확인하세요.

## 콘솔 액세스 필요

이건 보통 Microsoft 가족 그룹 안의 자녀 계정에서 나타납니다. 부모 계정이 [account.microsoft.com/family](https://account.microsoft.com/family)에서 자녀에게 Minecraft Java Edition을 허용해야 합니다. 허용 후 GDLauncher에서 로그아웃했다가 다시 로그인하세요.

## Xbox Live 오류

Xbox Live 실패는 보통 두 가지 중 하나입니다:

- Microsoft 계정의 국가/지역 설정이 Xbox Live를 허용하지 않음. [account.microsoft.com/profile](https://account.microsoft.com/profile)에서 조정하세요.
- 계정이 Xbox Live 이용 약관에 동의하지 않음. 동일 Microsoft 계정으로 [xbox.com](https://xbox.com)에 한 번 로그인해 약관에 동의한 다음 GDLauncher에서 다시 시도하세요.

## 계정 만료

Microsoft 리프레시 토큰이 만료되었거나 취소되었습니다 (다른 곳에서 계정 비밀번호를 바꾼 경우가 가장 흔합니다). GDLauncher는 'Account expired' 프롬프트를 표시하고 재인증을 제안합니다. **Settings → Accounts**에서 다시 로그인하세요.

## 그래도 해결되지 않으면

오류 메시지가 위 어느 것에도 맞지 않으면, 두 앱 레벨 로그를 [Discord](https://discord.gdlauncher.com)에서 공유해 주세요: `main.log` (Electron)과 최신 `__gdl_logs__/<timestamp>.log` (Rust 코어). 찾는 방법은 [Share App Logs](/guides/share-app-logs)를 참고하세요. 인증 흐름은 두 프로세스 사이를 오가기 때문에 둘 다 필요한 경우가 대부분입니다.
