import { JSX } from "solid-js"
import Navbar from "./Navbar"
import Sidebar from "./Sidebar"

interface LayoutProps {
  children: JSX.Element
}

export default function Layout(props: LayoutProps) {
  return (
    <div
      class="min-h-screen"
      style={`background-color: rgb(var(--darkSlate-900))`}
    >
      <Navbar />
      <div class="flex">
        <Sidebar />
        <main class="flex-1 p-6">{props.children}</main>
      </div>
    </div>
  )
}
