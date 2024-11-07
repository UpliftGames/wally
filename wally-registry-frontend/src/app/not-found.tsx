import { NotFoundSection } from "@/components/NotFoundSection"
import { Metadata } from "next"

export const metadata: Metadata = {
  title: "404: Not Found",
}

export default function NotFound() {
  return (
    <>
      <NotFoundSection />
    </>
  )
}
