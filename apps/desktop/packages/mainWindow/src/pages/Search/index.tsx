import ContentWrapper from "@/components/ContentWrapper"
import { Outlet } from "@solidjs/router"

export function Search() {
  return (
    <ContentWrapper zeroPadding>
      <div class="h-full overflow-hidden">
        <Outlet />
      </div>
    </ContentWrapper>
  )
}

export default Search
