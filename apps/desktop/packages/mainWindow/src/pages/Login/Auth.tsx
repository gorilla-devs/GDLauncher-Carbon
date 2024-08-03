import { Trans } from "@gd/i18n";

const Auth = () => {
  return (
    <div class="flex flex-col justify-center items-center text-center">
      <p class="text-darkSlate-50 text-sm max-w-90 mb-10">
        <Trans key="login.sign_in_with_microsoft_text" />
      </p>
    </div>
  );
};

export default Auth;
