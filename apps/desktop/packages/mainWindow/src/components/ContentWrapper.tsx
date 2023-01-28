import { createEffect, createSignal, useTransition } from "solid-js";

interface Props {
  children: any;
}
const ContentWrapper = (props: Props) => {
  const [isPending] = useTransition();
  const [isActuallyPending, setIsActuallyPending] = createSignal(false);

  let timer: ReturnType<typeof setTimeout>;

  createEffect(() => {
    console.log("isPending", isPending());
    if (isPending()) {
      if (timer) clearInterval(timer);
      timer = setInterval(() => {
        setIsActuallyPending(true);
      }, 300);
    } else {
      clearInterval(timer);
      setIsActuallyPending(false);
    }
  });

  return (
    <div class="w-full h-full max-h-[calc(100%-90px)] flex flex-1 justify-center overflow-auto box-border p-5 pb-0 text-white bg-shade-7">
      <div class="rounded-2xl rounded-b-none h-full w-full box-border bg-shade-8 overflow-auto relative">
        {isActuallyPending() ? "LOADING" : props.children}
      </div>
    </div>
  );
};

export default ContentWrapper;
