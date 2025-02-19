import { Trans } from "@gd/i18n"

const Auth = () => {
  return (
    <div class="flex flex-col items-center justify-center text-center">
      <p class="text-lightSlate-700 max-w-90 mb-10 text-sm">
        <Trans key="login.sign_in_with_microsoft_text" />
      </p>
    </div>
  )
}

export default Auth
