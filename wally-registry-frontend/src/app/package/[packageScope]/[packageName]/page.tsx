import PackageView from "@/components/PackageView"
import { Suspense } from "react"

export default function Package() {
  return (
    <>
      <Suspense>
        <PackageView />
      </Suspense>
    </>
  )
}
