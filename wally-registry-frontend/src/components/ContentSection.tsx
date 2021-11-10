import React from "react"
import styled, { css } from "styled-components"
// import albumImage from "../assets/hero/album.png";
import { breakMobile, isMobileSmall } from "../breakpoints"
import { ExternalLink, InternalLink } from "./InlineLink"
import { Heading } from "./Typography"

type ColorVariation = "light" | "dark" | "red" | "fade"
type PlacementVariation = "inline" | "floating"

interface StyledSectionProps {
  variation: ColorVariation
  placement: PlacementVariation
}

const themes = {
  dark: css`
    color: white;
    background: var(--wally-mauve);
  `,
  light: css`
    color: var(--wally-mauve);
    background: var(--wally-white);
  `,
  red: css`
    color: white;
    background: var(--wally-red);

    ${InternalLink}, ${ExternalLink} {
      border-bottom: 1px dotted white;
      color: white;

      &:hover {
        border-bottom: 1px solid white;
        text-decoration: none;
      }
    }
  `,
  fade: css`
    color: white;
    background: linear-gradient(
      180deg,
      rgba(0, 0, 0, 0.7) 0%,
      rgba(0, 0, 0, 1) 100%
    );
  `,
} as const

const floatingStyles = css`
  width: 90%;
  max-width: 1100px;
  margin: 0 auto;
  border-radius: var(--radius-small) var(--radius-small) var(--radius-large)
    var(--radius-small);
`

const StyledSection = styled.section<StyledSectionProps>`
  position: relative;
  flex-grow: 1;
  ${(props) => themes[props.variation]}
  ${(props) => (props.placement === "floating" ? floatingStyles : "")}
`

const ContentWrapper = styled.div`
  padding: 2.8rem 2rem;

  @media screen and (${isMobileSmall}) {
    padding: 2.8rem 1rem;
  }
`

const InnerContent = styled.article<{ narrow: boolean }>`
  max-width: ${(props) =>
    props.narrow ? `${breakMobile}px` : "var(--content-width)"};
  margin: 0 auto;

  ${Heading} {
    margin-bottom: 1rem;
  }

  ${Heading}:not(:first-child) {
    margin-top: 2rem;
  }

  & img {
    max-width: 100%;
  }
`

export default function ContentSection({
  children,
  variation,
  placement,
  narrow,
}: {
  children: React.ReactNode
  variation?: ColorVariation
  placement?: PlacementVariation
  narrow?: boolean
}) {
  return (
    <StyledSection
      variation={variation ?? "light"}
      placement={placement ?? "inline"}
    >
      <ContentWrapper>
        <InnerContent narrow={narrow ?? false}>{children}</InnerContent>
      </ContentWrapper>
    </StyledSection>
  )
}
