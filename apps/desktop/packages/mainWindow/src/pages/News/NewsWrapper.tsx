import ContentWrapper from "@/components/ContentWrapper"
import { NewsProvider } from "@/components/NewsContext"
import { JSX } from "solid-js"

export function NewsWrapper(props: { children?: JSX.Element }) {
  return (
    <NewsProvider>
      <ContentWrapper zeroPadding>
        {props.children}
      </ContentWrapper>
    </NewsProvider>
  )
}

export default NewsWrapper
