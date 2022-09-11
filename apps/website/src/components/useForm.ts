import { createStore } from "solid-js/store";

type FormFields = {
  email?: string;
  emailMc?: string;
  emailKofi?: string;
};

const useForm = () => {
  const [form, setForm] = createStore<FormFields>({
    email: "",
    emailMc: "",
    emailKofi: "",
  });

  const clearField = (fieldName: string) => {
    setForm({
      [fieldName]: "",
    });
  };

  const updateFormField = (fieldName: string) => (event: Event) => {
    const inputElement = event.currentTarget as HTMLInputElement;
    if (inputElement.type === "checkbox") {
      setForm({
        [fieldName]: !!inputElement.checked,
      });
    } else {
      setForm({
        [fieldName]: inputElement.value,
      });
    }
  };

  return { form, updateFormField, clearField };
};

export { useForm };
