import { NextRouter, withRouter } from "next/router"
import { useEffect } from "react"

function ScrollToTop({ router }: { router: NextRouter }) {
  const pathname = router.pathname
  useEffect(() => {
    window.scrollTo(0, 0)
    const checkbox = document.getElementById("nav-open") as HTMLInputElement
    checkbox.checked = false

    return () => {}
  }, [pathname])

  return null
}

export default withRouter(ScrollToTop)
