import Link from "next/link"
import React from "react"
import styled from "styled-components"

export const ExternalLink = styled.a`
  color: var(--wally-pink);

  &:hover {
    text-decoration: underline;
  }
`

export default function InlineLink({
  to,
  children,
}: {
  to: string
  children: React.ReactNode
}) {
  if (to.match(/^[a-z/]+:/)) {
    return (
      <ExternalLink href={to} target="_blank" rel="noopener">
        {children}
      </ExternalLink>
    )
  } else {
    return (
      <ExternalLink as={Link} href={to}>
        {children}
      </ExternalLink>
    )
  }
}
