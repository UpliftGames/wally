import styled from "styled-components"

const StyledMarkdownSection = styled.div`
  line-height: 1.8;
`

export function MarkdownSection({ children }: { children: React.ReactNode }) {
  return <StyledMarkdownSection>{children}</StyledMarkdownSection>
}
