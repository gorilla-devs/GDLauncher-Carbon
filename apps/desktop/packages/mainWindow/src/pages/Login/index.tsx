import { Button, toast, Spinner } from "@gd/ui"
import {
  createSignal,
  Switch,
  Match,
  createEffect,
  onMount,
  For,
  Show
} from "solid-js"
import Auth from "./Auth"
import CodeStep from "./CodeStep"
import fetchData from "./auth.login.data"
import { useRouteData, useSearchParams } from "@solidjs/router"
import { Trans } from "@gd/i18n"
import { rspc } from "@/utils/rspcClient"
import TermsAndConditions from "./TermsAndConditions"
import Logo from "/assets/images/gdlauncher_wide_logo_blue.svg"
import BackgroundVideo from "/assets/images/login_background.webm"
import { handleStatus } from "@/utils/login"
import { parseError } from "@/utils/helpers"
import GDLAccount from "./GDLAccount"
import GDLAccountCompletion from "./GDLAccountCompletion"
import { useGDNavigate } from "@/managers/NavigationManager"
import GDLAccountVerification from "./GDLAccountVerification"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import PrivacyNotice from "./PrivacyNotice"

export interface DeviceCodeObjectType {
  userCode: string
  link: string
  expiresAt: string
}

enum Steps {
  TermsAndConditions = 1,
  PrivacyNotice = 2,
  Auth = 3,
  CodeStep = 4,
  GDLAccount = 5,
  GDLAccountCompletion = 6,
  GDLAccountVerification = 7
}

export default function Login() {
  const routeData: ReturnType<typeof fetchData> = useRouteData()
  const [searchParams] = useSearchParams()

  const globalStore = useGlobalStore()

  const navigator = useGDNavigate()
  const [step, setStep] = createSignal<Steps>(Steps.TermsAndConditions)
  const [deviceCodeObject, setDeviceCodeObject] =
    createSignal<DeviceCodeObjectType | null>(null)
  const [loadingButton, setLoadingButton] = createSignal(false)
  const activeUuid = globalStore.currentlySelectedAccountUuid

  const gdlUser = rspc.createQuery(() => ({
    queryKey: ["account.peekGdlAccount", activeUuid.data!],
    enabled: !!activeUuid.data
  }))

  const saveGdlAccountMutation = rspc.createMutation(() => ({
    mutationKey: ["account.saveGdlAccount"]
  }))

  createEffect((prev) => {
    if (activeUuid.data && activeUuid.data !== prev) {
      gdlUser.refetch()
    }

    return activeUuid.data
  })

  const [recoveryEmail, setRecoveryEmail] = createSignal<string | null>(null)
  const [nickname, setNickname] = createSignal<string | null>(null)

  const [acceptedHashedEmail, setAcceptedHashedEmail] = createSignal(
    globalStore.settings.data?.hashedEmailAccepted
  )

  const [cooldown, setCooldown] = createSignal(0)

  let cooldownInterval: ReturnType<typeof setInterval> | undefined

  const rspcContext = rspc.useContext()

  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"]
  }))

  const deleteGDLAccountMutation = rspc.createMutation(() => ({
    mutationKey: ["account.removeGdlAccount"]
  }))

  const requestEmailChangeMutation = rspc.createMutation(() => ({
    mutationKey: ["account.requestEmailChange"]
  }))

  const registerGdlAccountMutation = rspc.createMutation(() => ({
    mutationKey: ["account.registerGdlAccount"]
  }))

  const changeGDLAccountNicknameMutation = rspc.createMutation(() => ({
    mutationKey: ["account.changeGdlAccountNickname"]
  }))

  const [isBackButtonVisible, setIsBackButtonVisible] = createSignal(false)

  const isGDLAccountSet = () =>
    globalStore.settings.data?.gdlAccountId ||
    globalStore.settings.data?.gdlAccountId === ""

  const accountEnrollFinalizeMutation = rspc.createMutation(() => ({
    mutationKey: ["account.enroll.finalize"]
  }))

  const nextStep = () => {
    if (step() < Steps.GDLAccountVerification) {
      setStep((prev) => prev + 1)
    }
  }

  const prevStep = () => {
    if (step() >= Steps.TermsAndConditions) {
      setStep((prev) => prev - 1)
    }
  }

  let sidebarRef: HTMLDivElement | undefined
  let backgroundBlurRef: HTMLDivElement | undefined
  let welcomeToTextRef: HTMLDivElement | undefined
  let gdlauncherTextRef: HTMLDivElement | undefined
  let loadingSpinnerRef: HTMLDivElement | undefined
  let videoRef: HTMLVideoElement | undefined

  async function transitionToLibrary() {
    return new Promise((resolve) => {
      if (backgroundBlurRef && globalStore.settings.data?.isFirstLaunch) {
        sidebarRef?.animate(
          [{ transform: "translateX(0%)" }, { transform: "translateX(-100%)" }],
          {
            duration: 500,
            easing: "linear",
            fill: "forwards"
          }
        )

        videoRef?.animate(
          [{ transform: "translateX(15%)" }, { transform: "translateX(0%)" }],
          {
            duration: 300,
            easing: "linear",
            fill: "forwards"
          }
        )

        backgroundBlurRef.animate([{ opacity: 0 }, { opacity: 1 }], {
          duration: 500,
          delay: 350,
          easing: "linear",
          fill: "forwards"
        })

        welcomeToTextRef?.animate([{ opacity: 0 }, { opacity: 1 }], {
          duration: 600,
          delay: 1100,
          easing: "linear",
          fill: "forwards"
        })

        gdlauncherTextRef?.animate([{ opacity: 0 }, { opacity: 1 }], {
          duration: 600,
          delay: 2300,
          easing: "linear",
          fill: "forwards"
        })

        setTimeout(() => {
          navigator.navigate("/library")
          resolve(null)
        }, 5000)
      } else {
        navigator.navigate("/library")
        resolve(null)
      }
    })
  }

  createEffect(() => {
    handleStatus(routeData.status, {
      onPolling: async (info) => {
        setDeviceCodeObject({
          userCode: info.userCode,
          link: info.verificationUri,
          expiresAt: info.expiresAt
        })
        if (routeData.status.data) {
          setStep(Steps.CodeStep)
          setLoadingButton(false)
        }
      },
      async onError(error) {
        if (error) toast.error(parseError(error))
        setStep(Steps.Auth)
        setLoadingButton(false)
      },
      async onComplete() {
        await accountEnrollFinalizeMutation.mutateAsync(undefined)

        const activeUuid = await rspcContext.client.query([
          "account.getActiveUuid"
        ])

        if (!activeUuid) {
          throw new Error("No active uuid")
        }

        const settings = await rspcContext.client.query([
          "settings.getSettings"
        ])

        // Already has GDL account
        if (
          settings.gdlAccountId !== null &&
          settings.gdlAccountId !== undefined
        ) {
          transitionToLibrary()
          return
        }

        const gdlUserPeek = await rspcContext.client.query([
          "account.peekGdlAccount",
          activeUuid
        ])

        if (gdlUserPeek?.email) {
          setRecoveryEmail(gdlUserPeek.email)
          setNickname(gdlUserPeek.nickname)
        }

        setStep(Steps.GDLAccount)
        setLoadingButton(false)
      }
    })
  })

  onMount(async () => {
    const activeUuid = await rspcContext.client.query(["account.getActiveUuid"])

    if (!globalStore.settings.data?.termsAndPrivacyAccepted) {
      setStep(Steps.TermsAndConditions)
      setIsBackButtonVisible(false)
      return requestAnimationFrame(() => {
        handleSidebarAnimation()
      })
    }

    const accounts = await rspcContext.client.query(["account.getAccounts"])

    if (
      !activeUuid ||
      accounts.length === 0 ||
      searchParams.addMicrosoftAccount
    ) {
      setStep(Steps.Auth)
      setIsBackButtonVisible(true)
      return requestAnimationFrame(() => {
        handleSidebarAnimation()
      })
    }

    if (isGDLAccountSet()) {
      transitionToLibrary()
    } else if (!isGDLAccountSet()) {
      setIsBackButtonVisible(true)
      setStep(Steps.GDLAccount)
    }

    return requestAnimationFrame(() => {
      handleSidebarAnimation()
    })
  })

  async function handleSidebarAnimation() {
    await new Promise((resolve) => setTimeout(resolve, 300))

    sidebarRef?.animate(
      [{ transform: "translateX(-100%)" }, { transform: "translateX(0)" }],
      {
        duration: 300,
        delay: 200,
        easing: "cubic-bezier(0.175, 0.885, 0.32, 1)",
        fill: "forwards"
      }
    )

    videoRef?.animate(
      [{ transform: "translateX(0)" }, { transform: "translateX(15%)" }],
      {
        duration: 300,
        delay: 200,
        easing: "cubic-bezier(0.175, 0.885, 0.32, 1)",
        fill: "forwards"
      }
    )

    loadingSpinnerRef?.animate([{ opacity: 1 }, { opacity: 0 }], {
      duration: 300,
      delay: 0,
      easing: "linear",
      fill: "forwards"
    })

    backgroundBlurRef?.animate([{ opacity: 1 }, { opacity: 0 }], {
      duration: 500,
      delay: 0,
      easing: "linear",
      fill: "forwards"
    })
  }

  let btnRef: HTMLDivElement | undefined

  function handleAnimationForward() {
    if (btnRef) {
      if (isBackButtonVisible()) return

      setIsBackButtonVisible(true)
      btnRef.animate(
        [
          { width: "0", margin: "0" },
          { width: "60%", margin: "0 1rem 0 0" }
        ],
        {
          duration: 300,
          easing: "cubic-bezier(0.175, 0.885, 0.32, 1.275)",
          fill: "forwards"
        }
      )
    }
  }

  function handleAnimationBackward() {
    if (btnRef && isBackButtonVisible() && step() === Steps.PrivacyNotice) {
      setIsBackButtonVisible(false)

      btnRef.animate(
        [
          { width: "60%", margin: "0 1rem 0 0" },
          { width: "0", margin: "0" }
        ],
        {
          duration: 300,
          easing: "cubic-bezier(0.175, 0.885, 0.32, 1.275)",
          fill: "forwards"
        }
      )
    }
  }

  const accountEnrollCancelMutation = rspc.createMutation(() => ({
    mutationKey: ["account.enroll.cancel"]
  }))

  const accountEnrollBeginMutation = rspc.createMutation(() => ({
    mutationKey: ["account.enroll.begin"],

    onError() {
      retryLogin()
    }
  }))

  const [retry, setRetry] = createSignal(0)

  const retryLogin = () => {
    while (retry() <= 3) {
      if (!routeData.status.data) {
        accountEnrollCancelMutation.mutate(undefined)
      }
      accountEnrollBeginMutation.mutate(undefined)
      setRetry((prev) => prev + 1)
    }
    if (retry() > 3) {
      toast.error("Something went wrong while logging in, try again!")
      if (routeData.status.data) {
        accountEnrollCancelMutation.mutate(undefined)
      }
    }
  }

  return (
    <div class="flex h-screen w-full" id="main-login-page">
      <div
        ref={sidebarRef}
        class="w-100 text-lightSlate-50 bg-darkSlate-800 z-1 absolute z-10 flex h-full -translate-x-full flex-col items-center rounded-md"
      >
        <div class="h-30 flex justify-center">
          <img class="w-60" src={Logo} />
        </div>
        <div class="mb-4 flex items-center justify-center gap-2 text-lg font-bold">
          <Switch>
            <Match when={step() === Steps.TermsAndConditions}>
              <div>
                <div>
                  <Trans key="login.titles.welcome_to_gdlauncher" />
                </div>
                <Show when={activeUuid?.data}>
                  <div>
                    <Trans key="login.renew" />
                  </div>
                </Show>
              </div>
            </Match>
            <Match when={step() === Steps.PrivacyNotice}>
              <Trans key="login.titles.we_value_privacy" />
            </Match>
            <Match when={step() === Steps.Auth}>
              <Trans key="login.titles.sign_in_with_microsoft" />
            </Match>
            <Match when={step() === Steps.CodeStep}>
              <i class="i-ri:microsoft-fill inline-block h-4 w-4" />
              <Trans key="login.titles.microsoft_code_step" />
            </Match>
            <Match when={step() === Steps.GDLAccount}>
              <Switch>
                <Match when={gdlUser.data}>
                  <Trans key="login.titles.sync_gdl_account" />
                </Match>
                <Match when={!gdlUser.data}>
                  <Trans key="login.titles.create_gdl_account" />
                </Match>
              </Switch>
            </Match>
            <Match when={step() === Steps.GDLAccountCompletion}>
              <Trans key="login.titles.linked_microsoft_account" />
            </Match>
            <Match when={step() === Steps.GDLAccountVerification}>
              <Trans key="login.titles.gdl_account_verification" />
            </Match>
          </Switch>
        </div>
        <div class="box-border flex h-auto w-full flex-1 overflow-y-auto px-4">
          <Switch>
            <Match when={step() === Steps.TermsAndConditions}>
              <TermsAndConditions />
            </Match>
            <Match when={step() === Steps.PrivacyNotice}>
              <PrivacyNotice />
            </Match>
            <Match when={step() === Steps.Auth}>
              <Auth />
            </Match>
            <Match when={step() === Steps.CodeStep}>
              <CodeStep
                nextStep={nextStep}
                prevStep={prevStep}
                deviceCodeObject={deviceCodeObject()}
                setDeviceCodeObject={setDeviceCodeObject}
              />
            </Match>
            <Match when={step() === Steps.GDLAccount}>
              <GDLAccount activeUuid={activeUuid.data} />
            </Match>
            <Match when={step() === Steps.GDLAccountCompletion}>
              <GDLAccountCompletion
                nextStep={nextStep}
                prevStep={prevStep}
                recoveryEmail={recoveryEmail()}
                setRecoveryEmail={setRecoveryEmail}
                nickname={nickname()}
                setNickname={setNickname}
                cooldown={cooldown()}
                acceptedHashedEmail={!!acceptedHashedEmail()}
                setAcceptedHashedEmail={setAcceptedHashedEmail}
              />
            </Match>
            <Match when={step() === Steps.GDLAccountVerification}>
              <GDLAccountVerification
                nextStep={nextStep}
                prevStep={prevStep}
                activeUuid={activeUuid.data}
                transitionToLibrary={transitionToLibrary}
              />
            </Match>
          </Switch>
        </div>

        <div class="box-border flex w-full flex-col items-center p-4">
          <div class="relative mb-4 flex justify-center gap-2">
            <div class="absolute left-0 top-1/2 h-4 w-full -translate-y-1/2 overflow-hidden rounded-lg">
              <div
                class="bg-darkSlate-400 absolute left-0 top-0 h-4 w-full rounded-lg"
                style={{
                  transform: `translateX(calc((-100% + ${(100 * step()) / 7}%) - ${(step() === Steps.TermsAndConditions ? 9 : 7) - step()}px)`,
                  transition:
                    "transform 0.3s cubic-bezier(0.175, 0.885, 0.32, 1.275)"
                }}
              />
            </div>

            <For each={new Array(7)}>
              {(_, i) => (
                <div
                  class="z-1 group flex h-6 w-4 items-center justify-center"
                  onClick={() => {
                    if (
                      i() + 1 < step() &&
                      (step() > Steps.CodeStep
                        ? i() + 1 > Steps.CodeStep
                        : step() >= Steps.Auth &&
                            searchParams.addMicrosoftAccount
                          ? false
                          : true)
                    ) {
                      setLoadingButton(false)
                      setStep(i() + 1)
                    }
                  }}
                >
                  <div
                    class="bg-lightSlate-900 h-2 w-2 rounded-full"
                    classList={{
                      "group-hover:bg-lightSlate-100":
                        i() + 1 < step() &&
                        (step() > Steps.CodeStep
                          ? i() + 1 > Steps.CodeStep
                          : step() >= Steps.Auth &&
                              searchParams.addMicrosoftAccount
                            ? false
                            : true)
                    }}
                  />
                </div>
              )}
            </For>
          </div>

          <div class="box-border flex w-full">
            <div
              ref={btnRef}
              class="overflow-hidden"
              style={{
                width: !isBackButtonVisible() ? "0" : "60%",
                margin: !isBackButtonVisible() ? "0" : "0 1rem 0 0"
              }}
            >
              <Button
                size="large"
                type="secondary"
                fullWidth
                onClick={async () => {
                  if (step() === Steps.GDLAccount) {
                    await saveGdlAccountMutation.mutateAsync("")
                    transitionToLibrary()
                  } else if (
                    (step() === Steps.Auth || step() === Steps.CodeStep) &&
                    globalStore.accounts.data?.length !== 0
                  ) {
                    navigator.navigate("/settings/accounts")
                  } else {
                    handleAnimationBackward()
                    prevStep()
                  }

                  setLoadingButton(false)
                }}
              >
                <Switch>
                  <Match
                    when={
                      step() === Steps.GDLAccount ||
                      ((step() === Steps.Auth || step() === Steps.CodeStep) &&
                        globalStore.accounts.data?.length !== 0)
                    }
                  >
                    <Trans key="general.skip" />
                    <i class="i-ri:skip-forward-line" />
                  </Match>
                  <Match when={step() !== Steps.GDLAccount}>
                    <i class="i-ri:arrow-left-line" />
                    <Trans key="general.back" />
                  </Match>
                </Switch>
              </Button>
            </div>
            <Button
              fullWidth
              variant="primary"
              size="large"
              disabled={
                step() === Steps.CodeStep ||
                step() === Steps.GDLAccountVerification ||
                (step() === Steps.GDLAccountCompletion &&
                  (!recoveryEmail()?.trim() || !nickname()?.trim()))
              }
              loading={
                loadingButton() || step() === Steps.GDLAccountVerification
              }
              onClick={async () => {
                handleAnimationForward()
                setLoadingButton(true)

                if (step() === Steps.TermsAndConditions) {
                  try {
                    await settingsMutation.mutateAsync({
                      termsAndPrivacyAccepted: {
                        Set: true
                      },
                      hashedEmailAccepted: {
                        Set: !!acceptedHashedEmail()
                      }
                    })
                  } catch (err) {
                    console.log(err)
                    toast.error("Error while accepting terms and conditions", {
                      description: "Check the console for more information."
                    })
                  }

                  setLoadingButton(false)

                  if (!searchParams.addMicrosoftAccount && activeUuid.data) {
                    navigator.navigate("/library")
                  } else {
                    nextStep()
                  }
                } else if (step() === Steps.PrivacyNotice) {
                  setLoadingButton(false)
                  nextStep()
                } else if (step() === Steps.Auth) {
                  if (!routeData.status.data) {
                    await accountEnrollBeginMutation.mutateAsync(undefined)
                  } else {
                    await accountEnrollCancelMutation.mutateAsync(undefined)
                    await accountEnrollBeginMutation.mutateAsync(undefined)
                  }
                } else if (step() === Steps.GDLAccount) {
                  const uuid = globalStore?.currentlySelectedAccountUuid?.data

                  if (!uuid) {
                    throw new Error("No active uuid")
                  }

                  try {
                    const existingGDLUser = await rspcContext.client.query([
                      "account.peekGdlAccount",
                      uuid
                    ])

                    if (existingGDLUser?.isEmailVerified) {
                      transitionToLibrary()
                      await saveGdlAccountMutation.mutateAsync(uuid)

                      return
                    } else if (
                      existingGDLUser &&
                      !existingGDLUser.isEmailVerified
                    ) {
                      setRecoveryEmail(existingGDLUser.email)
                      setNickname(existingGDLUser.nickname)
                      setStep(Steps.GDLAccountVerification)
                      return
                    }
                  } catch (e) {
                    console.error(e)
                  }

                  await deleteGDLAccountMutation.mutateAsync(undefined)
                  setLoadingButton(false)
                  nextStep()
                } else if (step() === Steps.GDLAccountCompletion) {
                  const uuid = globalStore?.currentlySelectedAccountUuid?.data

                  if (!uuid) {
                    throw new Error("No active uuid")
                  }

                  const email = recoveryEmail()?.trim()

                  if (!email) {
                    throw new Error("No recovery email")
                  }

                  if (!nickname()?.trim()) {
                    throw new Error("No nickname")
                  }

                  try {
                    const existingGDLUser = await rspcContext.client.query([
                      "account.peekGdlAccount",
                      uuid
                    ])

                    if (
                      existingGDLUser?.nickname &&
                      existingGDLUser.nickname !== nickname()
                    ) {
                      try {
                        await changeGDLAccountNicknameMutation.mutateAsync({
                          uuid,
                          nickname: nickname()!
                        })
                      } catch (e) {
                        console.error(e)
                        setLoadingButton(false)
                        return
                      }
                    }

                    if (
                      existingGDLUser?.email &&
                      existingGDLUser.email !== recoveryEmail()
                    ) {
                      try {
                        const result =
                          await requestEmailChangeMutation.mutateAsync({
                            uuid,
                            email: recoveryEmail()!
                          })

                        if (result.status === "success") {
                          setStep(Steps.GDLAccountVerification)
                          setLoadingButton(false)
                        } else if (result.status === "failed" && result.value) {
                          clearInterval(cooldownInterval)
                          cooldownInterval = undefined

                          setLoadingButton(false)
                          setCooldown(result.value)
                          setRecoveryEmail(existingGDLUser.email)

                          cooldownInterval = setInterval(() => {
                            setCooldown((prev) => prev - 1)

                            if (cooldown() <= 0) {
                              setCooldown(0)
                              clearInterval(cooldownInterval)
                              cooldownInterval = undefined
                            }
                          }, 1000)
                        }
                      } catch (e) {
                        console.error(e)
                        toast.error("Error while requesting email change", {
                          description: (e as any).message
                        })
                      }
                    } else if (existingGDLUser?.isEmailVerified) {
                      transitionToLibrary()
                    } else if (
                      existingGDLUser &&
                      !existingGDLUser.isEmailVerified
                    ) {
                      setRecoveryEmail(existingGDLUser.email)
                      setNickname(existingGDLUser.nickname)
                      setStep(Steps.GDLAccountVerification)
                    } else {
                      await registerGdlAccountMutation.mutateAsync({
                        email: recoveryEmail()!,
                        nickname: nickname()!,
                        uuid
                      })

                      await saveGdlAccountMutation.mutateAsync(uuid)

                      nextStep()
                    }
                  } catch (e) {
                    setLoadingButton(false)
                    console.error(e)
                  }
                }
              }}
            >
              <Switch>
                <Match when={step() === Steps.TermsAndConditions}>
                  <Trans key="login.agree_and_continue" />
                  <i class="i-ri:arrow-right-line" />
                </Match>
                <Match when={step() === Steps.PrivacyNotice}>
                  <Trans key="login.agree_and_continue" />
                  <i class="i-ri:arrow-right-line" />
                </Match>
                <Match
                  when={step() === Steps.GDLAccountCompletion && !gdlUser.data}
                >
                  <Trans key="login.register" />
                  <i class="i-ri:arrow-right-line" />
                </Match>
                <Match
                  when={
                    step() === Steps.GDLAccountCompletion &&
                    gdlUser.data &&
                    gdlUser.data.email !== recoveryEmail()
                  }
                >
                  <Trans key="login.request_email_change" />
                  <i class="i-ri:arrow-right-line" />
                </Match>
                <Match when={step() === Steps.GDLAccount && gdlUser.data}>
                  <Trans key="login.sync_gdl_account" />
                  <i class="i-ri:arrow-right-line" />
                </Match>
                <Match when={step() === Steps.Auth}>
                  <i class="i-ri:microsoft-fill h-4 w-4" />
                  <Trans key="login.sign_in" />
                </Match>
                <Match
                  when={step() === Steps.TermsAndConditions && activeUuid.data}
                >
                  <Trans key="instance_confirm" />
                </Match>
                <Match when={step() !== Steps.Auth}>
                  <Trans key="login.next" />
                  <i class="i-ri:arrow-right-line" />
                </Match>
              </Switch>
            </Button>
          </div>
        </div>
      </div>
      <div class="w-full flex-1">
        <div
          ref={backgroundBlurRef}
          class="z-1 absolute left-0 top-0 h-screen w-full bg-black/20 p-0"
          style={{
            "backdrop-filter": "blur(6px)"
          }}
        />
        <div class="z-1 absolute left-0 top-0 flex h-screen w-full flex-col items-center justify-center p-0 text-7xl font-bold leading-loose">
          <div ref={welcomeToTextRef} class="opacity-0">
            <Trans key="login.welcome_to" />
          </div>
          <div ref={gdlauncherTextRef} class="opacity-0">
            <Trans key="login.gdlauncher" />
          </div>
        </div>
        <div class="z-1 absolute left-0 top-0 flex h-screen w-full flex-col items-center justify-center p-0 text-7xl font-bold leading-loose">
          <div ref={loadingSpinnerRef}>
            <Spinner class="h-10 w-10" />
          </div>
        </div>
        <video
          ref={videoRef}
          class="h-screen w-full object-cover p-0"
          src={BackgroundVideo}
          autoplay
          muted
          loop
          playsinline
        />
      </div>
    </div>
  )
}
