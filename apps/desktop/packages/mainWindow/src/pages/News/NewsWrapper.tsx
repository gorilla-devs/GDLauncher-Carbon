import ContentWrapper from "@/components/ContentWrapper"
import { Outlet } from "@solidjs/router"
import { NewsProvider } from "@/components/NewsContext"

export function NewsWrapper() {
  return (
    <NewsProvider>
      <ContentWrapper zeroPadding>
        <Outlet />
      </ContentWrapper>
    </NewsProvider>
  )
}

export default NewsWrapper
