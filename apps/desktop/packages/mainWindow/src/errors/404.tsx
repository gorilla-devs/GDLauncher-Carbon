import { Trans } from "@gd/i18n";

export default function NotFound() {
  return (
    <div>
      <section class="text-gray-700 p-8">
        <h1 class="text-2xl font-bold">
          <Trans
            key="404_message"
            options={{
              defaultValue: "404: Not Found",
            }}
          />
        </h1>
        <p class="mt-4">
          <Trans
            key="its_gone"
            options={{
              defaultValue: "It's gone 😞",
            }}
          />
        </p>
      </section>
    </div>
  );
}
