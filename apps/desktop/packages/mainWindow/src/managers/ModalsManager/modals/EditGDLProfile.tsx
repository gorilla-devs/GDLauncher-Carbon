import { useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Button, Input, toast } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import {
  createEffect,
  createMemo,
  createSignal,
  Match,
  onCleanup,
  Show,
  Switch,
  untrack
} from "solid-js"
import { queryClient, rspc, apiFetch } from "@/utils/rspcClient"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import ImagePicker from "@/components/ImagePicker"
import { convertSecondsToHumanTime, blobToBase64 } from "@/utils/helpers"
import {
  sameGdlOwner,
  useSavedGdlOwner,
  type SavedGdlOwner,
  type SavedGdlOwnerStatus
} from "@/utils/savedGdlOwner"
import { getErrorCode } from "@/components/SharePreviewContent"

type AvatarUploadErrorKey =
  | "accounts:_trn_avatar_upload_rejected"
  | "accounts:_trn_avatar_upload_moderation_unavailable"
  | "accounts:_trn_avatar_upload_rate_limited"
  | "accounts:_trn_avatar_upload_too_large"
  | "accounts:_trn_avatar_upload_invalid_format"
  | "accounts:_trn_avatar_upload_failed"

function getAvatarUploadErrorKey(code: string | null): AvatarUploadErrorKey {
  switch (code) {
    case "IMAGE_REJECTED_BY_MODERATION":
      return "accounts:_trn_avatar_upload_rejected"
    case "MODERATION_UNAVAILABLE":
      return "accounts:_trn_avatar_upload_moderation_unavailable"
    case "MODERATION_RATE_LIMITED":
      return "accounts:_trn_avatar_upload_rate_limited"
    case "IMAGE_TOO_LARGE":
      return "accounts:_trn_avatar_upload_too_large"
    case "INVALID_IMAGE_FORMAT":
      return "accounts:_trn_avatar_upload_invalid_format"
    default:
      return "accounts:_trn_avatar_upload_failed"
  }
}

const EditGDLProfile = () => {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const globalStore = useGlobalStore()
  const ownerProof = useSavedGdlOwner()
  const [formOwner, setFormOwner] = createSignal<SavedGdlOwner>()
  const [nameBaseline, setNameBaseline] = createSignal("")
  const [emailBaseline, setEmailBaseline] = createSignal("")
  const [emailError, setEmailError] = createSignal<string | null>(null)
  const [verificationPending, setVerificationPending] = createSignal(false)

  // Form state
  const [displayName, setDisplayName] = createSignal("")
  const [recoveryEmail, setRecoveryEmail] = createSignal("")
  const [avatarPreview, setAvatarPreview] = createSignal<string | null>(null)
  const [avatarFilePath, setAvatarFilePath] = createSignal<string | null>(null)
  const [avatarDeleted, setAvatarDeleted] = createSignal(false)

  // Loading states
  const [isLoading, setIsLoading] = createSignal(false)
  const [avatarLoading, setAvatarLoading] = createSignal(false)

  // Error states
  const [displayNameError, setDisplayNameError] = createSignal<string | null>(
    null
  )

  // Cooldown states
  const [displayNameCooldown, setDisplayNameCooldown] = createSignal(0)
  const [emailCooldown, setEmailCooldown] = createSignal(0)
  const [verificationCooldown, setVerificationCooldown] = createSignal(0)

  let displayNameCooldownInterval: ReturnType<typeof setInterval> | undefined
  let emailCooldownInterval: ReturnType<typeof setInterval> | undefined
  let verificationCooldownInterval: ReturnType<typeof setInterval> | undefined

  // Mutations
  const changeDisplayNameMutation = rspc.createMutation(() => ({
    mutationKey: ["account.changeGdlAccountDisplayName"]
  }))

  const requestEmailChangeMutation = rspc.createMutation(() => ({
    mutationKey: ["account.requestEmailChange"]
  }))

  const uploadAvatarMutation = rspc.createMutation(() => ({
    mutationKey: ["account.uploadProfileIcon"]
  }))

  const deleteAvatarMutation = rspc.createMutation(() => ({
    mutationKey: ["account.deleteProfileIcon"]
  }))

  const requestNewVerificationTokenMutation = rspc.createMutation(() => ({
    mutationKey: ["account.requestNewVerificationToken"]
  }))

  const validGDLUser = () =>
    globalStore.gdlAccount.data?.status === "valid"
      ? globalStore.gdlAccount.data?.value
      : undefined

  // Helper to calculate remaining seconds from an absolute UTC timestamp
  const getRemainingSeconds = (
    timeoutAt: string | null | undefined,
    fallbackSeconds: number | null | undefined
  ): number => {
    if (timeoutAt) {
      const expiresAt = new Date(timeoutAt).getTime()
      const remaining = Math.floor((expiresAt - Date.now()) / 1000)
      return Math.max(0, remaining)
    }
    return fallbackSeconds && fallbackSeconds > 0 ? fallbackSeconds : 0
  }

  // Initialize form with current values
  let textFieldsInitialized = false
  createEffect(() => {
    const certified = ownerProof.capture()
    const user = certified?.profile
    if (user && (!formOwner() || ownerProof.current(formOwner()))) {
      // Seed the editable text fields only once. A background refetch of the
      // GDL account (e.g. after requesting a verification email or changing the
      // avatar in this same dialog) must not overwrite an unsaved display-name
      // or recovery-email edit the user is still typing.
      if (!textFieldsInitialized) {
        textFieldsInitialized = true
        setFormOwner(certified)
        setNameBaseline(user.displayName || "")
        setEmailBaseline(user.email || "")
        setDisplayName(user.displayName || "")
        setRecoveryEmail(user.email || "")
      }
      // Don't clobber a pending local selection (e.g. after an upload failure
      // where we keep the dialog open and want to preserve the user's choice).
      const hasPendingLocalAvatar = untrack(
        () => avatarFilePath() !== null || avatarDeleted()
      )
      if (!hasPendingLocalAvatar && user.profileIconUrl) {
        setAvatarPreview(user.profileIconUrl)
      }

      // Initialize cooldowns using absolute timestamps (with fallback to seconds)
      const displayNameRemaining = getRemainingSeconds(
        user.displayNameChangeTimeoutAt,
        user.displayNameChangeTimeout
      )
      if (displayNameRemaining > 0) {
        setDisplayNameCooldown(displayNameRemaining)
        startDisplayNameCooldownTimer()
      }

      const verificationRemaining = getRemainingSeconds(
        user.verificationTimeoutAt,
        user.verificationTimeout
      )
      if (verificationRemaining > 0) {
        setVerificationCooldown(verificationRemaining)
        startVerificationCooldownTimer()
      }

      const emailRemaining = getRemainingSeconds(
        user.emailChangeTimeoutAt,
        user.emailChangeTimeout
      )
      if (emailRemaining > 0) {
        setEmailCooldown(emailRemaining)
        startEmailCooldownTimer()
      }
    }
  })

  // What the dialog may do with the profile it shows. Routine refetches keep
  // the owner. If the owner is lost but the same account is still shown, Retry
  // can confirm it again; if another account is shown now, only reopening the
  // dialog loads it, so the typed text never lands on someone else. With no
  // valid account shown any more (it became invalid or was unlinked) there is
  // no Edit Profile to reopen, so the dialog points to Settings > Accounts.
  const ownerState = createMemo(
    (): SavedGdlOwnerStatus | "changed" | "gone" => {
      const owner = formOwner()
      if (owner && ownerProof.current(owner)) return "owned"
      const identity = ownerProof.identity()
      if (!identity) return "gone"
      if (owner && !sameGdlOwner(owner, identity)) return "changed"
      return owner ? "unavailable" : ownerProof.status()
    }
  )

  const [retryingOwner, setRetryingOwner] = createSignal(false)
  const retryOwner = async () => {
    if (retryingOwner()) return
    setRetryingOwner(true)
    try {
      await ownerProof.refetch()
    } finally {
      setRetryingOwner(false)
    }
    // Without a form owner the seeding effect captures on its own. With one,
    // only the very same account is admitted again, and the fields keep what
    // the user typed. A save still awaiting the old owner stays retired.
    const owner = formOwner()
    const fresh = ownerProof.capture()
    if (
      owner &&
      fresh &&
      sameGdlOwner(owner, fresh) &&
      !ownerProof.current(owner)
    )
      setFormOwner(fresh)
  }

  const startDisplayNameCooldownTimer = () => {
    if (displayNameCooldownInterval) clearInterval(displayNameCooldownInterval)
    displayNameCooldownInterval = setInterval(() => {
      setDisplayNameCooldown((prev) => {
        if (prev <= 1) {
          clearInterval(displayNameCooldownInterval)
          displayNameCooldownInterval = undefined
          return 0
        }
        return prev - 1
      })
    }, 1000)
  }

  const startEmailCooldownTimer = () => {
    if (emailCooldownInterval) clearInterval(emailCooldownInterval)
    emailCooldownInterval = setInterval(() => {
      setEmailCooldown((prev) => {
        if (prev <= 1) {
          clearInterval(emailCooldownInterval)
          emailCooldownInterval = undefined
          return 0
        }
        return prev - 1
      })
    }, 1000)
  }

  const startVerificationCooldownTimer = () => {
    if (verificationCooldownInterval)
      clearInterval(verificationCooldownInterval)
    verificationCooldownInterval = setInterval(() => {
      setVerificationCooldown((prev) => {
        if (prev <= 1) {
          clearInterval(verificationCooldownInterval)
          verificationCooldownInterval = undefined
          return 0
        }
        return prev - 1
      })
    }, 1000)
  }

  onCleanup(() => {
    if (displayNameCooldownInterval) clearInterval(displayNameCooldownInterval)
    if (emailCooldownInterval) clearInterval(emailCooldownInterval)
    if (verificationCooldownInterval)
      clearInterval(verificationCooldownInterval)
  })

  // Validation
  const isDisplayNameValid = createMemo(() => {
    const name = displayName().trim()
    return name.length >= 5 && name.length <= 20
  })

  const hasDisplayNameChanged = createMemo(() => {
    const currentName = nameBaseline()
    return displayName().trim() !== currentName
  })

  const hasEmailChanged = createMemo(() => {
    const currentEmail = emailBaseline()
    return recoveryEmail().trim() !== currentEmail
  })

  const hasAvatarChanged = createMemo(() => {
    return avatarFilePath() !== null || avatarDeleted()
  })

  const hasChanges = createMemo(() => {
    return hasDisplayNameChanged() || hasEmailChanged() || hasAvatarChanged()
  })

  const canSave = createMemo(() => {
    if (!ownerProof.current(formOwner()) || isLoading()) return false
    if (!hasChanges()) return false
    if (hasDisplayNameChanged() && !isDisplayNameValid()) return false
    if (hasDisplayNameChanged() && displayNameCooldown() > 0) return false
    if (hasEmailChanged() && emailCooldown() > 0) return false
    return true
  })

  // Avatar handling
  const handleAvatarSelect = async (filePath: string) => {
    setAvatarFilePath(filePath)
    setAvatarDeleted(false)

    // Load preview
    const response = await apiFetch(
      `/loadImage?path=${encodeURIComponent(filePath)}`
    )
    const blob = await response.blob()
    const b64 = (await blobToBase64(blob)) as string
    setAvatarPreview(
      `data:image/png;base64,${b64.substring(b64.indexOf(",") + 1)}`
    )
  }

  const handleAvatarRemove = () => {
    setAvatarDeleted(true)
    setAvatarFilePath(null)
    setAvatarPreview(null)
  }

  // Save changes
  const handleSave = async () => {
    const owner = formOwner()
    if (!owner || !canSave() || isLoading()) return
    const submitted = {
      name: displayName().trim(),
      email: recoveryEmail().trim(),
      changeName: hasDisplayNameChanged(),
      changeEmail: hasEmailChanged(),
      avatarPath: avatarFilePath(),
      deleteAvatar: avatarDeleted()
    }
    setIsLoading(true)
    try {
      if (!ownerProof.current(owner)) return
      if (submitted.deleteAvatar || submitted.avatarPath) {
        setAvatarLoading(true)
        try {
          if (submitted.deleteAvatar)
            await deleteAvatarMutation.mutateAsync(owner.uuid)
          else
            await uploadAvatarMutation.mutateAsync({
              uuid: owner.uuid,
              iconPath: submitted.avatarPath!
            })
          if (!ownerProof.current(owner)) return
          if (submitted.deleteAvatar && avatarDeleted()) setAvatarDeleted(false)
          if (submitted.avatarPath === avatarFilePath()) setAvatarFilePath(null)
          toast.success(
            t(
              submitted.deleteAvatar
                ? "accounts:_trn_avatar_delete_success"
                : "accounts:_trn_avatar_upload_success"
            )
          )
        } catch (err) {
          if (ownerProof.current(owner))
            toast.error(
              t(
                submitted.deleteAvatar
                  ? "accounts:_trn_avatar_delete_failed"
                  : getAvatarUploadErrorKey(getErrorCode(err))
              )
            )
          return
        } finally {
          setAvatarLoading(false)
        }
      }
      if (!ownerProof.current(owner)) return
      if (submitted.changeName) {
        const result = await changeDisplayNameMutation.mutateAsync({
          uuid: owner.uuid,
          displayName: submitted.name
        })
        if (!ownerProof.current(owner)) return
        if (result.status !== "success") {
          if (result.value && result.value > 0) {
            setDisplayNameCooldown(result.value)
            startDisplayNameCooldownTimer()
          }
          setDisplayNameError(t("accounts:_trn_display_name_change_failed"))
          return
        }
        setNameBaseline(submitted.name)
        queryClient.invalidateQueries({
          queryKey: ["account.getDisplayNameHistory", owner.friendCode]
        })
      }
      if (!ownerProof.current(owner)) return
      if (submitted.changeEmail) {
        const result = await requestEmailChangeMutation.mutateAsync({
          uuid: owner.uuid,
          email: submitted.email
        })
        if (!ownerProof.current(owner)) return
        if (result.status !== "success") {
          if (result.value && result.value > 0) {
            setEmailCooldown(result.value)
            startEmailCooldownTimer()
          }
          setEmailError(t("accounts:_trn_recovery_email_change_failed"))
          return
        }
        setEmailBaseline(submitted.email)
      }
      if (ownerProof.current(owner) && !hasChanges())
        modalsContext?.closeModal()
    } catch {
      // The shared mutation error path already supplies global feedback.
      if (ownerProof.current(owner)) {
        if (submitted.changeName && nameBaseline() !== submitted.name)
          setDisplayNameError(t("accounts:_trn_display_name_change_failed"))
        else if (submitted.changeEmail)
          setEmailError(t("accounts:_trn_recovery_email_change_failed"))
      }
    } finally {
      setIsLoading(false)
    }
  }

  const handleSendVerification = async () => {
    const owner = formOwner()
    if (
      !owner ||
      !ownerProof.current(owner) ||
      verificationPending() ||
      verificationCooldown() > 0
    )
      return
    setVerificationPending(true)
    try {
      const result = await requestNewVerificationTokenMutation.mutateAsync(
        owner.uuid
      )
      if (!ownerProof.current(owner)) return
      if (result.status === "success")
        toast.success(t("auth:_trn_login.verification_email_sent"))
      else if (result.value && result.value > 0) {
        setVerificationCooldown(result.value)
        startVerificationCooldownTimer()
      }
    } catch (err) {
      console.error("Verification request failed:", err)
    } finally {
      setVerificationPending(false)
    }
  }

  return (
    <ModalLayout
      title={t("accounts:_trn_edit_profile")}
      height="h-auto"
      width="w-150"
    >
      <div class="flex flex-col gap-6">
        <Switch>
          <Match when={ownerState() === "loading"}>
            <p class="text-lightSlate-400 m-0 text-sm">
              {t("auth:_trn_login.checking_gdl_account")}
            </p>
          </Match>
          <Match when={ownerState() === "unavailable"}>
            <p class="text-yellow-400">
              {t("accounts:_trn_gdl_account_unavailable")}
            </p>
            <Button
              type="secondary"
              loading={retryingOwner()}
              disabled={retryingOwner()}
              onClick={retryOwner}
            >
              <Trans key="general:_trn_retry" />
            </Button>
          </Match>
          <Match when={ownerState() === "changed"}>
            <p class="text-yellow-400">
              {t("accounts:_trn_gdl_account_changed_reopen")}
            </p>
          </Match>
          <Match when={ownerState() === "gone"}>
            <p class="text-yellow-400">
              {t("accounts:_trn_gdl_account_gone_close")}
            </p>
          </Match>
        </Switch>
        {/* Avatar + Display Name Section */}
        <div class="flex items-start gap-4">
          <ImagePicker
            imageUrl={avatarPreview}
            onSelect={handleAvatarSelect}
            onDelete={handleAvatarRemove}
            isLoading={avatarLoading}
            deletable={Boolean(
              avatarPreview() && validGDLUser()?.hasCustomAvatar
            )}
            confirmDelete={false}
            sizeClass="h-20 w-20"
            dialogTitle={t("accounts:_trn_select_avatar_image")}
            placeholderIcon="i-hugeicons:user"
          />

          {/* Display Name */}
          <div class="flex-1">
            <div class="text-lightSlate-200 mb-2 text-sm font-medium">
              <Trans key="accounts:_trn_display_name" />
            </div>
            <Input
              placeholder={t("auth:_trn_login.display_name")}
              value={displayName()}
              onInput={(e) => {
                setDisplayName(e.currentTarget.value)
                setDisplayNameError(null)
              }}
              disabled={
                !ownerProof.current(formOwner()) || !!displayNameCooldown()
              }
            />
            <p class="text-lightSlate-600 m-0 mt-1 text-xs">
              <Trans key="accounts:_trn_display_name_description" />
            </p>
            <Show when={displayNameError()}>
              <p class="m-0 mt-2 text-sm text-red-500">{displayNameError()}</p>
            </Show>
            <Show when={displayNameCooldown()}>
              <div class="mt-3 flex items-center gap-2 text-sm text-yellow-500">
                <div class="i-hugeicons:clock-01 text-base" />
                <Trans
                  key="accounts:_trn_display_name_change_cooldown"
                  options={{
                    time: convertSecondsToHumanTime(displayNameCooldown())
                  }}
                />
              </div>
            </Show>
          </div>
        </div>

        {/* Recovery Email Section */}
        <div>
          <div class="text-lightSlate-200 mb-2 text-sm font-medium">
            <Trans key="accounts:_trn_recovery_email" />
          </div>
          <Input
            placeholder={t("placeholders:_trn_email_example")}
            value={recoveryEmail()}
            onInput={(e) => {
              setRecoveryEmail(e.currentTarget.value)
              setEmailError(null)
            }}
            disabled={!ownerProof.current(formOwner()) || !!emailCooldown()}
          />
          <Show when={emailError()}>
            <p class="text-red-500">{emailError()}</p>
          </Show>
          <Show when={emailCooldown()}>
            <div class="mt-3 flex items-center gap-2 text-sm text-yellow-500">
              <div class="i-hugeicons:clock-01 text-base" />
              <Trans
                key="auth:_trn_login.new_email_request_wait"
                options={{
                  time: convertSecondsToHumanTime(emailCooldown())
                }}
              />
            </div>
          </Show>
        </div>

        {/* Email Verification Section */}
        <div>
          <div class="text-lightSlate-200 mb-2 text-sm font-medium">
            <Trans key="accounts:_trn_email_verification" />
          </div>
          <Show
            when={validGDLUser()?.isEmailVerified}
            fallback={
              <div class="flex flex-col gap-2">
                <div class="flex items-center gap-3">
                  <span class="text-yellow-400 text-sm">
                    <Trans key="accounts:_trn_not_verified" />
                  </span>
                  <Button
                    type="secondary"
                    size="small"
                    disabled={
                      !ownerProof.current(formOwner()) ||
                      verificationPending() ||
                      !!verificationCooldown()
                    }
                    onClick={handleSendVerification}
                  >
                    <div class="i-hugeicons:mail-send-01" />
                    <Trans key="accounts:_trn_verify_email" />
                  </Button>
                </div>
                <Show when={verificationCooldown()}>
                  <div class="flex items-center gap-2 text-sm text-yellow-500">
                    <div class="i-hugeicons:clock-01 text-base" />
                    <Trans
                      key="accounts:_trn_cannot_request_deletion_for_time"
                      options={{
                        time: convertSecondsToHumanTime(verificationCooldown())
                      }}
                    />
                  </div>
                </Show>
              </div>
            }
          >
            <span class="flex items-center gap-1 text-green-400">
              <div class="i-hugeicons:tick-02" />
              <Trans key="accounts:_trn_verified" />
            </span>
          </Show>
        </div>

        {/* Buttons */}
        <div class="flex justify-between border-t border-darkSlate-600 pt-4">
          <Button onClick={() => modalsContext?.closeModal()} type="secondary">
            <Trans key="accounts:_trn_cancel" />
          </Button>
          <Button
            type="primary"
            disabled={!canSave()}
            loading={isLoading()}
            onClick={handleSave}
          >
            <Trans key="accounts:_trn_save_changes" />
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default EditGDLProfile
