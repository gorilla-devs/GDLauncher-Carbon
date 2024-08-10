import { Trans, useTransContext } from "@gd/i18n";
import { Input } from "@gd/ui";

interface Props {
  nextStep: () => void;
  prevStep: () => void;
  recoveryEmail: string | null;
  setRecoveryEmail: (_: string | null) => void;
}

const GDLAccountCompletion = (props: Props) => {
  const [t] = useTransContext();

  return (
    <div class="flex-1 w-full flex flex-col justify-between items-center text-center gap-5 p-10">
      <div class="flex flex-col w-full gap-4">
        <div class="text-lg">
          <Trans key="login.enter_your_recovery_email" />
        </div>
        <Input
          placeholder={t("login.recovery_email")}
          class="w-full"
          value={props.recoveryEmail || ""}
          onSearch={(value) => {
            props.setRecoveryEmail(value);
          }}
        />
        <div class="text-sm text-lightSlate-500">
          <Trans key="login.recovery_email_description" />
        </div>
      </div>
    </div>
  );
};

export default GDLAccountCompletion;
