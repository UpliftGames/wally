import { useEffect } from "react"
import { withRouter } from "react-router-dom"

function ScrollToTop({ history }: { history: any }) {
  useEffect(() => {
    const unlisten = history.listen(() => {
      window.scrollTo(0, 0)
      const checkbox = document.getElementById("nav-open") as HTMLInputElement
      checkbox.checked = false
    })
    return () => {
      unlisten()
    }
  }, [])

  return null
}

export default withRouter(ScrollToTop)
