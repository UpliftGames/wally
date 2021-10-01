import React from "react"
import { Link } from "react-router-dom"
import styled from "styled-components"

export const ExternalLink = styled.a`
  color: var(--wally-pink);

  &:hover {
    text-decoration: underline;
  }
`

export const InternalLink = ExternalLink.withComponent(Link)

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
    return <InternalLink to={to}>{children}</InternalLink>
  }
}
