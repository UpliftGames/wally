import { ButtonLink } from "./Button"
import { Heading } from "./Typography"

export default function NotFoundMessage({
  errorMessage,
}: {
  errorMessage: string
}) {
  return (
    <>
      <Heading>Lost Your Way?</Heading>

      <p>{errorMessage}</p>

      <ButtonLink to="/">Return Home</ButtonLink>
    </>
  )
}
