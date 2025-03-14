import ContentWrapper from "@/components/ContentWrapper"
import { Outlet } from "@solidjs/router"

export function NewsWrapper() {
  return (
    <ContentWrapper zeroPadding>
      <Outlet />
    </ContentWrapper>
  )
}

export default NewsWrapper
