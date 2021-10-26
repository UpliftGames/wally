import React from "react"
import { Helmet } from "react-helmet"
import ContentSection from "../components/ContentSection"
import NotFoundMessage from "../components/NotFoundMessage"

export default function NotFound() {
  return (
    <>
      <Helmet>
        <title>404: Not Found</title>
      </Helmet>
      <ContentSection variation="light">
        <NotFoundMessage errorMessage="HTTP 404: Page Not Found" />
      </ContentSection>
    </>
  )
}
