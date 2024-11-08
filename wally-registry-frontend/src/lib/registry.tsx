"use client"

import { useServerInsertedHTML } from "next/navigation"
import React, { useState } from "react"
import { ServerStyleSheet, StyleSheetManager } from "styled-components"

export default function StyledComponentsRegistry({
  children,
}: {
  children: React.ReactNode
}) {
  // Only create stylesheet once with lazy initial state
  // x-ref: https://reactjs.org/docs/hooks-reference.html#lazy-initial-state
  const [styledComponentsStyleSheet] = useState(() => new ServerStyleSheet())

  useServerInsertedHTML(() => {
    const styles = styledComponentsStyleSheet.getStyleElement()
    styledComponentsStyleSheet.instance.clearTag()
    return <>{styles}</>
  })

  if (typeof window !== "undefined") return <>{children}</>

  return (
    <StyleSheetManager
      sheet={styledComponentsStyleSheet.instance}
      //   shouldForwardProp={shouldForwardProp} If there are many styled-components collision issues, a shouldForwardProp function can be added to solve some v5 migration problems.
      // https://styled-components.com/docs/api#shouldforwardprop
    >
      {children}
    </StyleSheetManager>
  )
}
