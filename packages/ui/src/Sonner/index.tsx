import { Toaster as Sonner, toast } from "somoto"

export { toast }

// Inject CSS utility class to suppress toast entrance animation
// Use className="toast-no-animate" on any toast that updates content without needing animation
if (typeof document !== "undefined") {
  const style = document.createElement("style")
  style.textContent = `
    [data-somoto-toast].toast-no-animate {
      transition: none !important;
      opacity: 1 !important;
      --y: translateY(0) !important;
    }
  `
  document.head.appendChild(style)
}

const SuccessIcon = () => (
  <svg
    width="16"
    height="16"
    viewBox="0 0 24 24"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
  >
    <path
      d="M20 6L9 17l-5-5"
      stroke="rgb(var(--green-500))"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  </svg>
)

const ErrorIcon = () => (
  <svg
    width="16"
    height="16"
    viewBox="0 0 24 24"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
  >
    <circle
      cx="12"
      cy="12"
      r="10"
      stroke="rgb(var(--red-500))"
      stroke-width="2"
    />
    <line
      x1="15"
      y1="9"
      x2="9"
      y2="15"
      stroke="rgb(var(--red-500))"
      stroke-width="2"
      stroke-linecap="round"
    />
    <line
      x1="9"
      y1="9"
      x2="15"
      y2="15"
      stroke="rgb(var(--red-500))"
      stroke-width="2"
      stroke-linecap="round"
    />
  </svg>
)

const WarningIcon = () => (
  <svg
    width="16"
    height="16"
    viewBox="0 0 24 24"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
  >
    <path
      d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"
      stroke="rgb(var(--yellow-500))"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
    <line
      x1="12"
      y1="9"
      x2="12"
      y2="13"
      stroke="rgb(var(--yellow-500))"
      stroke-width="2"
      stroke-linecap="round"
    />
    <circle cx="12" cy="17" r="1" fill="rgb(var(--yellow-500))" />
  </svg>
)

const InfoIcon = () => (
  <svg
    width="16"
    height="16"
    viewBox="0 0 24 24"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
  >
    <circle
      cx="12"
      cy="12"
      r="10"
      stroke="rgb(var(--primary-500))"
      stroke-width="2"
    />
    <path
      d="m9,12 h3 v5 h3"
      stroke="rgb(var(--primary-500))"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
    <circle cx="12" cy="8" r="1" fill="rgb(var(--primary-500))" />
  </svg>
)

const CloseIcon = () => <div class="i-hugeicons:cancel-01 h-4 w-4" />

export const Toaster = (props: Parameters<typeof Sonner>[0]) => {
  return (
    <Sonner
      position="bottom-left"
      richColors
      theme="dark"
      closeButton
      icons={{
        success: <SuccessIcon />,
        error: <ErrorIcon />,
        warning: <WarningIcon />,
        info: <InfoIcon />,
        close: <CloseIcon />
      }}
      toastOptions={{
        classNames: {
          toast: "group toast border shadow-lg",
          title: "font-medium",
          description: "opacity-75",
          success: "bg-darkSlate-800 text-lightSlate-50 border-green-500/20",
          error: "bg-darkSlate-800 text-lightSlate-50 border-red-500/20",
          warning: "bg-darkSlate-800 text-lightSlate-50 border-yellow-500/20",
          info: "bg-darkSlate-800 text-lightSlate-50 border-primary-500/20",
          default: "bg-darkSlate-800 text-lightSlate-50 border-darkSlate-600",
          closeButton:
            "!bg-darkSlate-700/50 !text-lightSlate-300 hover:!bg-darkSlate-600 hover:!text-lightSlate-50 !border-0 !shadow-none !rounded-md !p-1 transition-colors",
          actionButton:
            "transition-all duration-100 ease-spring hover:brightness-110 font-bold rounded-md",
          cancelButton:
            "transition-all duration-100 ease-spring hover:brightness-110 hover:bg-darkSlate-500 font-bold rounded-md"
        },
        style: {
          "background-color": "rgb(var(--darkSlate-800))",
          color: "rgb(var(--lightSlate-50))",
          "border-color": "rgb(var(--darkSlate-600))"
        },
        actionButtonStyle: {
          "background-color": "rgb(var(--primary-500))",
          color: "rgb(var(--lightSlate-50))",
          padding: "0.75rem 1.25rem",
          "border-radius": "0.375rem",
          "font-weight": "700",
          border: "none",
          cursor: "pointer",
          transition: "all 100ms ease-spring",
          "font-family": "inherit"
        },
        cancelButtonStyle: {
          "background-color": "rgb(var(--darkSlate-600))",
          color: "rgb(var(--lightSlate-300))",
          padding: "0.75rem 1.25rem",
          "border-radius": "0.375rem",
          "font-weight": "700",
          border: "2px solid rgb(var(--darkSlate-600))",
          cursor: "pointer",
          transition: "all 100ms ease-spring",
          "font-family": "inherit"
        }
      }}
      {...props}
    />
  )
}
