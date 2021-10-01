import React from "react"
import styled from "styled-components"

declare const process: any

export default function Img({
  src,
  className,
  ...rest
}: {
  src: string
  className?: string
  [index: string]: any
}) {
  if (src.endsWith(".png") && process.env.NODE_ENV === "production") {
    const fileName = src.substr(0, src.length - 4)
    return (
      <picture>
        <source srcSet={fileName + ".avif"} type="image/avif" />
        <source srcSet={fileName + ".webp"} type="image/webp" />
        <img src={src} {...rest} className={className} />
      </picture>
    )
  } else {
    return (
      <picture>
        <img src={src} {...rest} className={className} />
      </picture>
    )
  }
}

export const BgImg = styled(Img)<{ align?: string }>`
  pointer-events: none;
  width: 100%;
  height: 100%;
  z-index: -1;
  object-fit: cover;
  object-position: ${(props) => props.align ?? "center"};
  position: absolute;
`