import { Trans } from "@gd/i18n"

export default function NotFound() {
  return (
    <div>
      <section class="p-8 text-gray-700">
        <h1 class="text-2xl font-bold">
          <Trans
            key="errors:_trn_404_message"
            options={{
              defaultValue: "404: Not Found"
            }}
          />
        </h1>
        <p class="mt-4">
          <Trans
            key="errors:_trn_its_gone"
            options={{
              defaultValue: "It's gone 😞"
            }}
          />
        </p>
      </section>
    </div>
  )
}
