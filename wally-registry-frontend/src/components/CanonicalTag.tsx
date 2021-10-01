import React from "react"
import { Helmet } from "react-helmet"
import { withRouter } from "react-router-dom"

function CanonicalTag({ history }: { history: any }) {
  return (
    <Helmet>
      <link
        rel="canonical"
        href={`https://wally.run/${history.location.pathname}`}
      />
    </Helmet>
  )
}

export default withRouter(CanonicalTag)
