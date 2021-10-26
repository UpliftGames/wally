import styled, { css } from "styled-components"
import { isMobile, isMobileSmall } from "../breakpoints"

type ColorVariation = "red"

const variations = {
  red: css`
    color: var(--wally-red);
  `,
}

export const Heading = styled.h1<{ variation?: ColorVariation }>`
  font-size: 2.5rem;
  font-weight: 900;
  text-transform: uppercase;
  letter-spacing: 0.3px;
  line-height: 1.2;

  @media screen and (${isMobile}) {
    font-size: 2rem;
  }

  ${(props) => props.variation && variations[props.variation]}
`

export const SuperHeading = styled.h1`
  font-size: 4rem;
  font-weight: 900;
  font-style: oblique;
  color: white;
  text-transform: uppercase;

  @media screen and (${isMobile}) {
    font-size: 3rem;
  }
`

export const ParagraphStyle = css`
  margin: 1rem 0;
  font-size: 1.1rem;
  letter-spacing: 0.3px;
  line-height: 1.5;

  b {
    font-weight: bold;
  }

  i {
    font-style: italic;
  }

  u {
    text-decoration: underline;
  }
`

export const Paragraph = styled.p`
  ${ParagraphStyle}
`

export const ResponsiveParagraph = styled.p`
  ${ParagraphStyle}

  @media screen and (${isMobile}) {
    font-size: 1rem;
  }

  @media screen and (${isMobileSmall}) {
    font-size: 0.9rem;
  }
`

export const Code = styled.code`
  padding: 0.2em 0.4em;
  margin: 0;
  font-size: 85%;
  color: white;
  background-color: var(--wally-grey-transparent);
  border-radius: 6px;
  font-family: monospace;
`
