import { JSX, children } from "solid-js"

interface ComponentDemoProps {
  title: string
  description?: string
  children: JSX.Element
  code?: string
  spacing?: string
}

export default function ComponentDemo(props: ComponentDemoProps) {
  const c = children(() => props.children)

  return (
    <div class={props.spacing || "mb-8"}>
      <div class="mb-4">
        <h2
          class="text-2xl font-bold mb-2"
          style={`color: rgb(var(--lightSlate-50))`}
        >
          {props.title}
        </h2>
        {props.description && (
          <p style={`color: rgb(var(--lightSlate-300))`}>{props.description}</p>
        )}
      </div>

      <div
        class="border rounded-lg overflow-hidden"
        style={`border-color: rgb(var(--darkSlate-600))`}
      >
        <div class="p-6" style={`background-color: rgb(var(--darkSlate-700))`}>
          {c()}
        </div>

        {props.code && (
          <div
            class="border-t"
            style={`border-color: rgb(var(--darkSlate-600)); background-color: rgb(var(--darkSlate-800))`}
          >
            <div class="px-6 py-4">
              <div class="flex items-center justify-between mb-2">
                <span
                  class="text-sm font-medium font-mono"
                  style={`color: rgb(var(--lightSlate-300)); font-family: var(--font-mono)`}
                >
                  Code
                </span>
                <button
                  onClick={() => navigator.clipboard.writeText(props.code!)}
                  class="text-sm transition-colors hover:underline"
                  style={`color: rgb(var(--primary-400)); :hover { color: rgb(var(--primary-300)); }`}
                >
                  Copy
                </button>
              </div>
              <pre
                class="text-sm overflow-x-auto font-mono"
                style={`color: rgb(var(--lightSlate-100)); font-family: var(--font-mono)`}
              >
                <code>{props.code}</code>
              </pre>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
