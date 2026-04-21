import { useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Button, Input, toast } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import {
  createEffect,
  createMemo,
  createSignal,
  onCleanup,
  Show,
  untrack
} from "solid-js"
import { queryClient, rspc, port } from "@/utils/rspcClient"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import ImagePicker from "@/components/ImagePicker"
import { convertSecondsToHumanTime, blobToBase64 } from "@/utils/helpers"
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
  createEffect(() => {
    const user = validGDLUser()
    if (user) {
      setDisplayName(user.displayName || "")
      setRecoveryEmail(user.email || "")
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
    const currentName = validGDLUser()?.displayName || ""
    return displayName().trim() !== currentName
  })

  const hasEmailChanged = createMemo(() => {
    const currentEmail = validGDLUser()?.email || ""
    return recoveryEmail().trim() !== currentEmail
  })

  const hasAvatarChanged = createMemo(() => {
    return avatarFilePath() !== null || avatarDeleted()
  })

  const hasChanges = createMemo(() => {
    return hasDisplayNameChanged() || hasEmailChanged() || hasAvatarChanged()
  })

  const canSave = createMemo(() => {
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
    const response = await fetch(
      `http://127.0.0.1:${port}/loadImage?path=${encodeURIComponent(filePath)}`
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
    const uuid = globalStore?.currentlySelectedAccountUuid?.data
    if (!uuid) return

    setIsLoading(true)

    try {
      // Handle avatar changes first. If one fails we bail out before closing
      // the dialog so the user can correct the image / try a different file.
      if (avatarDeleted()) {
        setAvatarLoading(true)
        try {
          await deleteAvatarMutation.mutateAsync(uuid)
          toast.success(t("accounts:_trn_avatar_delete_success"))
        } catch (err) {
          console.error("Avatar deletion failed:", err)
          toast.error(t("accounts:_trn_avatar_delete_failed"))
          setAvatarLoading(false)
          setIsLoading(false)
          return
        }
        setAvatarLoading(false)
      } else if (avatarFilePath()) {
        setAvatarLoading(true)
        try {
          await uploadAvatarMutation.mutateAsync({
            uuid,
            iconPath: avatarFilePath()!
          })
          toast.success(t("accounts:_trn_avatar_upload_success"))
        } catch (err) {
          console.error("Avatar upload failed:", err)
          toast.error(t(getAvatarUploadErrorKey(getErrorCode(err))))
          setAvatarLoading(false)
          setIsLoading(false)
          return
        }
        setAvatarLoading(false)
      }

      // Handle display name change
      if (hasDisplayNameChanged() && isDisplayNameValid()) {
        const result = await changeDisplayNameMutation.mutateAsync({
          uuid,
          displayName: displayName().trim()
        })

        if (result?.status === "success") {
          queryClient.invalidateQueries({
            queryKey: ["account.getDisplayNameHistory"]
          })
        } else if (result?.status === "failed" && result.value) {
          setDisplayNameCooldown(result.value)
          startDisplayNameCooldownTimer()
          setDisplayNameError(t("accounts:_trn_display_name_change_failed"))
          setIsLoading(false)
          return
        }
      }

      // Handle email change
      if (hasEmailChanged() && recoveryEmail().trim()) {
        const result = await requestEmailChangeMutation.mutateAsync({
          uuid,
          email: recoveryEmail().trim()
        })

        if (result.status === "failed" && result.value) {
          setEmailCooldown(result.value)
          startEmailCooldownTimer()
          setIsLoading(false)
          return
        }
      }

      modalsContext?.closeModal()
    } catch (err) {
      console.error("Save failed:", err)
    }

    setIsLoading(false)
  }

  // Send verification email
  const handleSendVerification = async () => {
    const uuid = globalStore?.currentlySelectedAccountUuid?.data
    if (!uuid) return

    try {
      const result = await requestNewVerificationTokenMutation.mutateAsync(uuid)
      if (result.status === "success") {
        toast.success(t("auth:_trn_login.verification_email_sent"))
      } else if (result.status === "failed" && result.value) {
        setVerificationCooldown(result.value)
        startVerificationCooldownTimer()
      }
    } catch (err) {
      console.error("Verification request failed:", err)
    }
  }

  return (
    <ModalLayout
      title={t("accounts:_trn_edit_profile")}
      height="h-auto"
      width="w-150"
    >
      <div class="flex flex-col gap-6">
        {/* Avatar + Display Name Section */}
        <div class="flex items-start gap-4">
          <ImagePicker
            imageUrl={avatarPreview}
            onSelect={handleAvatarSelect}
            onDelete={handleAvatarRemove}
            isLoading={avatarLoading}
            deletable={Boolean(avatarPreview() && validGDLUser()?.hasCustomAvatar)}
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
              disabled={!!displayNameCooldown()}
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
            onInput={(e) => setRecoveryEmail(e.currentTarget.value)}
            disabled={!!emailCooldown()}
          />
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
                    disabled={!!verificationCooldown()}
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
