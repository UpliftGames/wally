"use client"

import ContentSection from "@/components/ContentSection"
import { MarkdownSection } from "@/components/MarkdownSection"
import Policies from "@/markdown/Policies.mdx"

export default function InstallPage() {
  return (
    <ContentSection>
      <MarkdownSection>
        <Policies />
      </MarkdownSection>
    </ContentSection>
  )
}
