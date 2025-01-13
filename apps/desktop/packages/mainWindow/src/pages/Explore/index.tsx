import ContentWrapper from "@/components/ContentWrapper"
import { Outlet } from "@solidjs/router"

export function Explore() {
  return (
    <ContentWrapper>
      <Outlet />
    </ContentWrapper>
  )
}

export default Explore
