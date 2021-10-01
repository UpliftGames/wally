import React from "react"
import { Link } from "react-router-dom"
import styled from "styled-components"
import SocialLinks from "./SocialLinks"

const StyledFooter = styled.footer`
  background: var(--wally-red);
  padding: 0.5rem;
  color: white;

  display: flex;
  flex-direction: column;
  align-items: center;
`

const StyledSocialLinks = styled(SocialLinks)`
  font-size: 2rem;
  letter-spacing: 0.5rem;
  justify-content: center;
`

const CopyrightLine = styled.p`
  font-size: 0.8rem;
  margin-top: 1rem;
  text-align: center;

  small {
    font-size: 0.7rem;
  }

  a {
    color: white;

    &:hover {
      text-decoration: underline;
    }
  }
`

const FooterLinks = styled.div`
  display: flex;
  flex-wrap: wrap;
  justify-content: center;

  & > a {
    position: relative;
    padding: 0 0.65rem;
    letter-spacing: 0.3px;
    font-size: 1.6rem;
    font-weight: 900;

    &:not(:last-child)::after {
      content: " ";
      display: block;
      position: absolute;
      height: 1.4rem;
      border-right: 2px solid white;
      right: 0;
      top: 52%;
      transform: translateY(-50%);
    }

    &:hover {
      color: var(--wally-mauve);
    }
  }
`

const links: [React.ReactNode, string][] = [
  ["Install", "/install/"],
  ["Policies", "/policies/"],
]

export default function Footer() {
  return (
    <StyledFooter>
      <StyledSocialLinks variation="light" />
      <FooterLinks>
        {links.map(([text, url]) => (
          <Link key={url} to={url}>
            {text}
          </Link>
        ))}
      </FooterLinks>
    </StyledFooter>
  )
}
