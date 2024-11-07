"use client"

import ContentSection from "@/components/ContentSection"
import { MarkdownSection } from "@/components/MarkdownSection"
import Install from "@/markdown/Install.mdx"

export default function InstallPage() {
  return (
    <ContentSection>
      <MarkdownSection>
        <Install />
      </MarkdownSection>
    </ContentSection>
  )
}
