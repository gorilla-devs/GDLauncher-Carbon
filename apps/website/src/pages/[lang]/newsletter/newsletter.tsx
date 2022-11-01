import Button from "@/components/Button";
import Input from "@/components/Input";
import LoadingSpinner from "@/components/LoadingSpinner";
import { ADD_USER_ENDPOINT } from "@/constants";
import { useTranslations } from "@/i18n/utils";
import { createSignal, Show } from "solid-js";
import { useForm } from "../../../components/useForm";

const WaitList = ({ pathname }: { pathname: string }) => {
  // const { form, updateFormField } = useForm();
  // const [error, setError] = createSignal("");
  // const [loading, setLoading] = createSignal(false);
  // const [success, setSuccess] = createSignal("");
  // const t = useTranslations(pathname);

  // const addUser = async (body: any) => {
  //   return await fetch(ADD_USER_ENDPOINT, {
  //     method: "POST",
  //     headers: {
  //       Accept: "application/json",
  //       "Content-Type": "application/json",
  //     },
  //     body: JSON.stringify(body),
  //   });
  // };

  // const handleSubmit = async (event: Event) => {
  //   event.preventDefault();
  //   const obj: any = {};
  //   setError("");
  //   setSuccess("");
  //   setLoading(true);

  //   if (form.email) {
  //     obj["email"] = form.email;
  //     const res = await addUser(obj);
  //     if (res.status === 400) {
  //       setError(t("newsletter.error_400"));
  //     } else {
  //       setSuccess(t("newsletter.success"));
  //     }
  //   } else {
  //     setError(t("newsletter.error_missing_info"));
  //   }
  //   setLoading(false);
  // };

  return (
    <div class="pt-10 pb-10 lg:pb-0 h-screen relative flex flex-col justify-center items-center">
    </div>
  );
};

export default WaitList;
