import InlineLink from "@/components/InlineLink"
import { Heading, Paragraph } from "@/components/Typography"
import type { MDXComponents } from "mdx/types"
import styled, { css } from "styled-components"

const MarkdownH2 = styled.h2`
  font-size: 1.5rem;
  font-weight: 600;
`

const MarkdownH3 = styled.h3`
  font-size: 1.2rem;
  font-weight: 600;
`

const MarkdownH4 = styled.h4`
  font-weight: 600;
`

const MarkdownH5 = styled.h5`
  font-weight: 600;
`

const MarkdownH6 = styled.h6`
  font-weight: 600;
`

const MarkdownStrong = styled.strong`
  font-weight: bold;
`

const MarkdownEm = styled.em`
  font-style: italic;
`

const MarkdownDel = styled.del`
  text-decoration: line-through;
`

const textBlockStyles = css`
  margin: 1rem 0;
`

const listStyles = css`
  padding-left: 1.5em;
  line-height: 1.4;
`

const MarkdownUl = styled.ul`
  ${listStyles}
  ${textBlockStyles}
`

const MarkdownOl = styled.ol`
  ${listStyles}
  ${textBlockStyles}
`

const MarkdownLi = styled.li`
  margin: 0.5rem 0;
  font-size: 1.1rem;
`

const MarkdownBlockquote = styled.blockquote`
  ${textBlockStyles}
  padding-left: 1em;
  margin-left: 0.25em;
  border-left: 4px solid rgba(0, 0, 0, 0.2);
`

const MarkdownTable = styled.table``

const MarkdownTHead = styled.thead``

const MarkdownTBody = styled.tbody``

const MarkdownTr = styled.tr``

const MarkdownTd = styled.td``

const MarkdownTh = styled.th``

const MarkdownPre = styled.pre`
  ${textBlockStyles}
  background-color: rgba(0, 0, 0, 0.05);
  padding: 0.5em;
  line-height: 1.2;
  overflow-x: auto;
  font-family: monospace;
`

const MarkdownCode = styled.code`
  font-family: monospace;
`

const MarkdownLink = styled(InlineLink)`
  color: var(--color-highlight-1);

  &:hover,
  &:focus {
    text-decoration: underline;
  }
`

export function useMDXComponents(components: MDXComponents): MDXComponents {
  return {
    h1: Heading,
    h2: MarkdownH2,
    h3: MarkdownH3,
    h4: MarkdownH4,
    h5: MarkdownH5,
    h6: MarkdownH6,
    p: Paragraph,
    strong: MarkdownStrong,
    em: MarkdownEm,
    del: MarkdownDel,
    ul: MarkdownUl,
    ol: MarkdownOl,
    li: MarkdownLi,
    blockquote: MarkdownBlockquote,
    table: MarkdownTable,
    thead: MarkdownTHead,
    tbody: MarkdownTBody,
    tr: MarkdownTr,
    td: MarkdownTd,
    th: MarkdownTh,
    pre: MarkdownPre,
    code: MarkdownCode,
    a: ({ href, ...rest }) => <MarkdownLink to={href || ""} {...rest} />,
    ...components,
  }
}
