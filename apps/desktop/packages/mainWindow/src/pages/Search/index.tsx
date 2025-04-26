import ContentWrapper from "@/components/ContentWrapper"
import { Outlet } from "@solidjs/router"

export function Search() {
  return (
    <ContentWrapper zeroPadding>
      <Outlet />
    </ContentWrapper>
  )
}

export default Search
