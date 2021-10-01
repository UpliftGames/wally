import { MDXProvider } from "@mdx-js/react"
import React from "react"
import InlineLink from "./components/InlineLink"
import { Heading, Paragraph } from "./components/Typography"
import style from "./MDXComponents.css"

const mdxify = (Component) => (props) => (
  <Component className={style[Component]} {...props} />
)

const primitives = [
  "h2",
  "h3",
  "h4",
  "h5",
  "h6",
  "strong",
  "em",
  "del",
  "ul",
  "ol",
  "li",
  "thematicBreak",
  "blockquote",
  "table",
  "thead",
  "tbody",
  "tr",
  "td",
  "th",
  "pre",
  "code",
  "inlineCode",
  "img",
]
const components = {
  wrapper: ({ children }) => <div className={style.wrapper}>{children}</div>,
  h1: Heading,
  p: Paragraph,
  a: ({ href, ...rest }) => <InlineLink to={href} {...rest} />,
}

for (const primitive of primitives) {
  components[primitive] = mdxify(primitive)
}

export default function MDXComponents({ children }) {
  return <MDXProvider components={components}>{children}</MDXProvider>
}
