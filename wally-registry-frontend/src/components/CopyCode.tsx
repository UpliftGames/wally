import { useState } from "react"
import styled, { css, keyframes } from "styled-components"
import Icon from "./Icon"

const jumpKeyframes = keyframes`
  0%{
    -webkit-transform: translateY(0);
    transform: translateY(0);
  }
  20%{
    -webkit-transform: translateY(-8px);
    transform: translateY(-8px);
  }
  30%{
    -webkit-transform: translateY(0);
    transform: translateY(0);
  }
  40%{
    -webkit-transform: translateY(-4px);
    transform: translateY(-4px);
  }
  60%{
    -webkit-transform: translateY(0);
    transform: translateY(0);
  }
  100%{
    -webkit-transform: translateY(0);
    transform: translateY(0);
  }
`

const jumpAnimation = css`
  animation: ${jumpKeyframes} 0.9s both;
`

const CopiedMessage = styled.span`
  margin: 0 auto;
`

const CopyIcon = styled.div`
  visibility: none;
  opacity: 0;
  transition: opacity 0.3s;
  color: white;
`

const CopyCodeButton = styled.button<{ $recentlyCopied: boolean }>`
  display: flex;
  justify-content: space-between;
  align-items: center;
  text-align: left;
  width: 100%;
  padding: 3px 3px 3px 5px;
  font-size: 0.9rem;
  color: white;
  background-color: var(--wally-grey-transparent);
  border-radius: var(--radius-small);
  cursor: pointer;
  word-break: break-word;
  white-space: pre-line;

  ${(props) => props.$recentlyCopied && jumpAnimation}

  &:hover ${CopyIcon} {
    visibility: visible;
    opacity: 1;
  }
`

export default function CopyCody({
  packageName,
  version,
}: {
  packageName: string
  version: string
}) {
  const [recentlyCopied, setRecentlyCopied] = useState(false)

  const copyContent = `${
    packageName.split("/")[1]
  } = "${packageName}@${version}"`

  const copyToClipBoard = (text: string) => {
    navigator.clipboard.writeText(text)

    setRecentlyCopied(true)

    setTimeout(() => {
      setRecentlyCopied(false)
    }, 900)
  }

  return (
    <CopyCodeButton
      $recentlyCopied={recentlyCopied}
      onClick={() => copyToClipBoard(copyContent)}
    >
      <>
        {recentlyCopied ? (
          <CopiedMessage>Copied! 👍</CopiedMessage>
        ) : (
          <code>{copyContent}</code>
        )}
        <CopyIcon>
          <Icon icon="copy" />
        </CopyIcon>
      </>
    </CopyCodeButton>
  )
}
