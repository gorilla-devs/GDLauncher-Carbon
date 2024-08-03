import { Input } from "@gd/ui";

interface Props {
  nextStep: () => void;
  prevStep: () => void;
  recoveryEmail: string | null;
  setRecoveryEmail: (value: string | null) => void;
}

const GDLAccountCompletion = (props: Props) => {
  return (
    <div class="flex-1 w-full flex flex-col justify-between items-center text-center gap-5 p-10">
      <div class="flex flex-col w-full gap-4">
        <div class="text-lg">Enter your recovery email</div>
        <Input
          placeholder="Enter your recovery email"
          class="w-full"
          value={props.recoveryEmail || ""}
          onSearch={(value) => {
            props.setRecoveryEmail(value);
          }}
        />
        <div class="text-sm text-lightSlate-500">
          This email should be different from your Microsoft account, to ensure
          you are able to recover your data in case you lose access to it.
        </div>
      </div>
    </div>
  );
};

export default GDLAccountCompletion;
