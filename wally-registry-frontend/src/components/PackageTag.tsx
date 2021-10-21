import React from "react"
import { Link } from "react-router-dom"
import styled, { keyframes } from "styled-components"

type SizeVariation = "small" | "large"
type WidthVariation = "narrow" | "wide"

const ArrowHoverKeyfames = keyframes`
  0% { right: 17px; }
  50% { right: 20px; }
  100% { right: 17px; }
`

const StyledBlip = styled.div<{ size: SizeVariation; width: WidthVariation }>`
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  width: ${(props) => (props.width === "narrow" ? "25rem" : "100%")};
  max-width: 90vw;
  border-radius: var(--radius-small);
  border-top-right-radius: ${(props) =>
    props.size === "large" ? "var(--radius-circle) 50%" : "35px 50%"};
  border-bottom-right-radius: ${(props) =>
    props.size === "large" ? "var(--radius-circle) 50%" : "35px 50%"};
  background: var(--wally-mauve);
  color: white;
  font-size: 0.9rem;
  font-weight: 300;
  letter-spacing: -0.4px;
  position: relative;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;

  &:any-link:hover {
    background: var(--wally-red);

    ::before {
      animation: ${ArrowHoverKeyfames};
      animation-duration: 0.5s;
      animation-fill-mode: forwards;
      animation-iteration-count: infinite;
    }
  }

  ::before {
    position: absolute;
    content: "";
    display: inline-block;
    width: ${(props) => (props.size === "large" ? "40px" : "20px")};
    height: ${(props) => (props.size === "large" ? "40px" : "20px")};
    border: 0 solid var(--wally-white);
    border-right-width: 0.2em;
    border-top-width: 0.2em;
    transform: rotate(45deg);
    margin-right: 0.5em;
    right: 17px;
    top: calc(
      50% - ${(props) => (props.size === "large" ? "25px" : "13px")}
    ); // The px value comes from the Pythagorean Theorem applied to the rotated element and it's corner.
  }
`

const LinkBlip = StyledBlip.withComponent(Link)

const TopRow = styled.div`
  display: flex;
  justify-content: space-between;
  margin: 0.2rem 0.5rem 0.3rem 0.5rem;
`

const Title = styled.span`
  color: var(--wally-white);
  font-size: 1.1rem;
  white-space: nowrap;
  flex-basis: 200%;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
`

const RowWrapper = styled.div<{ $inset: boolean }>`
  width: 90%;

  & > *:first-child {
    flex-grow: 1;
  }

  & > p {
    margin: 0.5rem;
    font-size: 0.9rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  & > picture,
  & > img {
    max-width: 100%;
    line-height: unset;
  }

  & > picture > img {
    width: 100%;
  }

  & > picture {
    margin: ${(props) => (props.$inset ? "0.4rem" : "initial")};
  }
`

const BottomRow = styled.div`
  display: flex;
  justify-content: space-between;
  margin: 0.2rem 0.5rem 0.3rem 0.5rem;
`

const Author = styled.span`
  color: rgba(255, 255, 255, 0.7);
  font-size: 0.8rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  flex-basis: 200%;
`

export default function PackageBlip({
  size = "large",
  width = "narrow",
  children,
  title,
  version,
  uniqueId,
  className,
  linkTo,
  inset,
}: {
  size?: SizeVariation
  width?: WidthVariation
  children?: React.ReactNode
  title?: string
  author?: string
  version?: string
  uniqueId?: string
  className?: string
  linkTo?: string
  inset?: boolean
}) {
  const Component: React.FC<any> = linkTo ? LinkBlip : StyledBlip

  return (
    <Component
      className={className}
      to={`package/${linkTo}`}
      size={size}
      width={width}
    >
      <RowWrapper $inset={inset ?? false}>
        <TopRow>
          <Title>{title}</Title>
        </TopRow>

        {size === "large" ? <>{children}</> : <></>}

        <BottomRow>
          <Author>{version}</Author>
        </BottomRow>
      </RowWrapper>
    </Component>
  )
}
