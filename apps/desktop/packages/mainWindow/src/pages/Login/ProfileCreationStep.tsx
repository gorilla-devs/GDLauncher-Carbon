import {
  Show,
  createSignal,
  createMemo,
  createEffect,
  onCleanup
} from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import { rspc } from "@/utils/rspcClient"

interface ProfileCreationStepProps {
  accessToken: string
  nextStep: () => void
  onValidationChange?: (isValid: boolean) => void
  onPendingChange?: (isPending: boolean) => void
  onSubmitReady?: (submitFn: () => void) => void
}

const ProfileCreationStep = (props: ProfileCreationStepProps) => {
  const [t] = useTransContext()
  const [username, setUsername] = createSignal("")
  const [checking, setChecking] = createSignal(false)
  const [available, setAvailable] = createSignal<boolean | null>(null)
  const [error, setError] = createSignal<string | null>(null)

  // Username validation
  const isValid = createMemo(() => {
    const name = username()
    if (name.length === 0) return false
    if (name.length < 3 || name.length > 16) return false
    // Only letters, numbers, and underscores
    return /^[a-zA-Z0-9_]+$/.test(name)
  })

  // Check username availability mutation
  const checkAvailability = rspc.createMutation(() => ({
    mutationKey: ["account.checkUsernameAvailable"]
  }))

  // Debounced availability check
  createEffect(() => {
    const name = username()

    // Reset states
    setAvailable(null)
    setError(null)

    if (!isValid()) return

    // Debounce the check
    const timer = setTimeout(async () => {
      setChecking(true)
      try {
        const result = await checkAvailability.mutateAsync({
          accessToken: props.accessToken,
          username: name
        })
        setAvailable(result)
      } catch (err) {
        console.error("Error checking username:", err)
        setError(t("profile_creation.check_failed"))
      } finally {
        setChecking(false)
      }
    }, 500)

    onCleanup(() => clearTimeout(timer))
  })

  // Resume enrollment mutation
  const resumeEnrollment = rspc.createMutation(() => ({
    mutationKey: ["account.enroll.resume"]
  }))

  // Create profile mutation
  const createProfile = rspc.createMutation(() => ({
    mutationKey: ["account.createProfile"],
    onSuccess: async () => {
      // Resume the enrollment flow after profile creation
      try {
        await resumeEnrollment.mutateAsync(undefined)
        props.nextStep()
      } catch (err) {
        console.error("Error resuming enrollment:", err)
        setError(t("profile_creation.created_but_failed_continue"))
      }
    },
    onError: (err: any) => {
      console.error("Error creating profile:", err)
      if (err.message?.includes("InvalidUsername")) {
        setError(t("profile_creation.username_invalid"))
      } else if (err.message?.includes("NameNotAvailable")) {
        setError(t("profile_creation.username_unavailable"))
        setAvailable(false)
      } else {
        setError(t("profile_creation.create_failed"))
      }
    }
  }))

  const handleCreateProfile = () => {
    setError(null)
    createProfile.mutate({
      accessToken: props.accessToken,
      username: username()
    })
  }

  // Expose submit function to parent
  createEffect(() => {
    if (props.onSubmitReady) {
      props.onSubmitReady(handleCreateProfile)
    }
  })

  // Notify parent of validation state changes
  createEffect(() => {
    const canSubmit = isValid() && available() === true
    props.onValidationChange?.(canSubmit)
  })

  // Notify parent of pending state changes
  createEffect(() => {
    props.onPendingChange?.(createProfile.isPending)
  })

  return (
    <div class="flex w-full flex-col items-center gap-6 text-center">
      <div class="i-hugeicons:user-add-02 h-16 w-16 text-primary-400" />

      <div>
        <h2 class="text-lightSlate-50 mb-2 text-xl font-bold">
          <Trans key="profile_creation.title" />
        </h2>
        <p class="text-lightSlate-400 text-sm">
          <Trans key="profile_creation.description" />
        </p>
      </div>

      <div class="w-full max-w-80">
        <input
          type="text"
          value={username()}
          onInput={(e) => setUsername(e.currentTarget.value)}
          placeholder={t("profile_creation.username_placeholder")}
          class="border-darkSlate-600 bg-darkSlate-700 text-lightSlate-50 placeholder:text-lightSlate-700 w-full rounded-lg border px-4 py-3 focus:border-primary-500 focus:outline-none"
          maxLength={16}
          disabled={createProfile.isPending}
        />

        {/* Validation feedback */}
        <div class="mt-2 min-h-6 text-left text-sm">
          <Show when={username() && !isValid()}>
            <p class="text-red-400">
              <Trans key="profile_creation.invalid_format" />
            </p>
          </Show>
          <Show when={isValid() && checking()}>
            <p class="text-lightSlate-500 flex items-center">
              <span class="i-hugeicons:loading-03 h-3 w-3 mr-2 inline animate-spin" />
              <Trans key="profile_creation.checking" />
            </p>
          </Show>
          <Show when={isValid() && !checking() && available() === true}>
            <p class="text-green-400">
              ✓ <Trans key="profile_creation.available" />
            </p>
          </Show>
          <Show when={isValid() && !checking() && available() === false}>
            <p class="text-red-400">
              ✗ <Trans key="profile_creation.taken" />
            </p>
          </Show>
        </div>

        {/* Requirements */}
        <div class="text-lightSlate-600 mt-4 text-left text-xs">
          <p class="mb-2 font-semibold">
            <Trans key="profile_creation.requirements" />:
          </p>
          <ul class="list-disc space-y-1 pl-5">
            <li
              classList={{
                "text-green-400":
                  username().length >= 3 && username().length <= 16
              }}
            >
              <Trans key="profile_creation.requirement_length" />
            </li>
            <li
              classList={{
                "text-green-400": /^[a-zA-Z0-9_]*$/.test(username())
              }}
            >
              <Trans key="profile_creation.requirement_characters" />
            </li>
            <li>
              <Trans key="profile_creation.requirement_no_profanity" />
            </li>
          </ul>
        </div>
      </div>

      <Show when={error()}>
        <p class="text-red-400 text-sm">{error()}</p>
      </Show>
    </div>
  )
}

export default ProfileCreationStep
