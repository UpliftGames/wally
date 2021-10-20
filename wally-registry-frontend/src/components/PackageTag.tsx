import React from "react"
import { Link } from "react-router-dom"
import styled, { keyframes } from "styled-components"

const ArrowHoverKeyfames = keyframes`
  0% { right: 17px; }
  50% { right: 20px; }
  100% { right: 17px; }
`

const StyledBlip = styled.div`
  font-size: 0.9rem;
  border-radius: var(--radius-small);
  border-top-right-radius: var(--radius-circle) 50%;
  border-bottom-right-radius: var(--radius-circle) 50%;
  background: var(--wally-mauve);
  color: white;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  font-weight: 300;
  width: 25rem;
  letter-spacing: -0.4px;
  max-width: 90vw;
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
    width: 40px;
    height: 40px;
    border: 0 solid var(--wally-white);
    border-right-width: 0.2em;
    border-top-width: 0.2em;
    transform: rotate(45deg);
    margin-right: 0.5em;
    right: 17px;
    top: calc(
      50% - 25px
    ); // 25px comes from the Pythagorean Theorem applied to the rotated element and it's corner. Leg c is 40px.
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
  children,
  title,
  author,
  version,
  uniqueId,
  className,
  linkTo,
  inset,
}: {
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
    <Component className={className} to={`package/${linkTo}`}>
      <RowWrapper $inset={inset ?? false}>
        <TopRow>
          <Title>{title}</Title>
        </TopRow>

        {children}

        <BottomRow>
          <Author>{version}</Author>
        </BottomRow>
      </RowWrapper>
    </Component>
  )
}
