"use client"

import ContentSection from "@/components/ContentSection"
import NotFoundMessage from "@/components/NotFoundMessage"

export function NotFoundSection() {
  return (
    <ContentSection variation="light">
      <NotFoundMessage errorMessage="HTTP 404: Page Not Found" />
    </ContentSection>
  )
}
