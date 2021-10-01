import React from "react"
import { Link } from "react-router-dom"
import styled from "styled-components"

export const Button = styled.button`
  display: inline-block;
  padding: 0.6rem 1.5rem;
  background: var(--wally-brown);
  border-radius: var(--radius-small);
  color: var(--wally-white);
  margin: 0.5rem;
  text-transform: uppercase;
  border: 1px solid var(--wally-brown);
  font-weight: 700;
  font-size: 1.2rem;

  &:first-child {
    margin-left: 0;
  }

  &:last-child {
    margin-right: 0;
  }

  &:hover {
    background: var(--wally-gold);
    border: 1px solid var(--wally-gold);
    color: var(--wally-mauve);
  }
`
const ButtonLinkInternal = Button.withComponent(Link)
const ButtonLinkExternal = Button.withComponent("a")

export function ButtonLink({
  to,
  children,
}: {
  to: string
  children: React.ReactNode
}) {
  if (to.match(/^[a-z/]+:/)) {
    return (
      <ButtonLinkExternal href={to} target="_blank">
        {children}
      </ButtonLinkExternal>
    )
  } else {
    return <ButtonLinkInternal to={to}>{children}</ButtonLinkInternal>
  }
}
