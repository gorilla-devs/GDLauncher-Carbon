import napi from "@/utils/napi";
import { useNavigate } from "@solidjs/router";
import { createSignal } from "solid-js";
// import { Button } from "@gd/ui";

export default function Home() {
  const [count, setCount] = createSignal(0);
  const [value, setValue] = createSignal<number>();

  const navigate = useNavigate();

  return (
    <section class="bg-gray-100 text-gray-700 p-8">
      <h1 class="text-2xl font-bold">Home</h1>
      <p class="mt-4">This is the home page.</p>

      <div class="flex items-center space-x-2">
        <button
          class="border rounded-lg px-2 border-gray-900"
          onClick={() => setCount(count() - 1)}
        >
          -
        </button>

        {/* <Button>Hi</Button> */}

        <output class="p-10px">Count: {count}</output>

        <button
          class="border rounded-lg px-2 border-gray-900"
          onClick={() => setCount(count() + 1)}
        >
          +
        </button>
      </div>
      <button onClick={() => navigate("?m=myModal")}>Open modal</button>
      <div />
      <button
        onClick={async () => {
          console.log(await napi.fibonacci(1, 2));
          // const res = await napi.fibonacci(1, 2);
          // setValue(res);
        }}
      >
        Compute value
      </button>
      <div />
      {value}
      <div />
    </section>
  );
}
