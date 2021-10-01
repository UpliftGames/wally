import React from "react"
import styled, { css } from "styled-components"
import socialConfig from "../config/socialLinks.json"
import { IconsId } from "../types/icons"
import Icon from "./Icon"

const socialLinks: [IconsId, string, React.CSSProperties][] = [
  // ["twitter", socialConfig.twitter, {}],
  // ["instagram", socialConfig.instagram, {}],
  // ["youtube", socialConfig.youTube, {}],
  // ["linkedin", socialConfig.linkedIn, {}],
  ["github", socialConfig.gitHub, {}],
  [
    "discord",
    socialConfig.discord,
    {
      fontSize: "1.2em",
      marginRight: "-0.3rem",
      marginTop: "0.2em",
    },
  ],
]

type ColorVariation = "light"

const variations = {
  light: css`
    color: white;

    a {
      padding: 0.6rem 0.45rem 0 0.5rem;

      &:hover {
        color: var(--wally-mauve);
      }
    }
  `,
}

const StyledSocialLinks = styled.div<{ variation?: ColorVariation }>`
  display: flex;
  align-items: stretch;
  flex-wrap: wrap;
  align-items: center;
  font-size: 1.6rem;

  a {
    padding: 0.6rem 0.45rem 0 0.5rem;

    &:hover {
      color: var(--wally-red);
    }
  }

  ${(props) => props.variation && variations[props.variation]}
`

export default function SocialLinks({ ...rest }) {
  return (
    <StyledSocialLinks {...rest}>
      {socialLinks.map(([image, url, style]) => (
        <a href={url} key={url} style={style}>
          <Icon icon={image} />
        </a>
      ))}
    </StyledSocialLinks>
  )
}
