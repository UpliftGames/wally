import React from "react"
import { Helmet } from "react-helmet"
import { ButtonLink } from "../components/Button"
import ContentSection from "../components/ContentSection"
import { Heading } from "../components/Typography"

export default function NotFound() {
  return (
    <>
      <Helmet>
        <title>404: Not Found</title>
      </Helmet>
      <ContentSection variation="light">
        <Heading>Lost Your Way?</Heading>

        <ButtonLink to="/">Return Home</ButtonLink>
        <p>HTTP 404: Page Not Found</p>
      </ContentSection>
    </>
  )
}
