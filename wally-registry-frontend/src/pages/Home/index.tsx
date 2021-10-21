import React from "react"
import styled from "styled-components"
import { ButtonLink } from "../../components/Button"
import CallToActionContainer from "../../components/CallToActionContainer"
import ContentSection from "../../components/ContentSection"
import PackageTag from "../../components/PackageTag"
import { Code, Heading, Paragraph } from "../../components/Typography"
import mockPackages from "../../mocks/packages.mock"

const Flex = styled.div`
  display: flex;
  flex-wrap: wrap;
  gap: 2rem;
  justify-content: space-around;
  margin: 1.5rem 0;
`

const PopularPackages = () => (
  <>
    {[...mockPackages].map((popPackage, index) => (
      <PackageTag
        size="small"
        key={index}
        uniqueId={popPackage.package.name
          .substr(0, 15)
          .toLowerCase()
          .replace(/[^a-z]/gi, "")}
        title={popPackage.package.name.split("/")[1]}
        author={popPackage.package.authors.join(" ")}
        version={popPackage.package.version}
        linkTo={popPackage.package.name.split("/")[1]}
      >
        <p>{popPackage.package.description}</p>
      </PackageTag>
    ))}
  </>
)

export default function Home() {
  return (
    <>
      <ContentSection variation="red" placement="floating">
        <Heading>Wally, a package manager for Roblox</Heading>
        <Paragraph>
          Wally is a package manager for Roblox inspired by Cargo (Rust) and npm
          (JavaScript). It brings the familiar, community-oriented world of
          sharing code from other communities into the Roblox ecosystem.
        </Paragraph>
        <Paragraph>
          Wally has two pieces that work together: a command line tool named{" "}
          <Code>wally</Code> and a registry server that hosts packages. Most
          users will only interact with the command line tool, but both are
          available in this repository.
        </Paragraph>

        <CallToActionContainer>
          <ButtonLink to="install">Install Wally</ButtonLink>
        </CallToActionContainer>
      </ContentSection>

      <ContentSection variation="light">
        <Heading>Popular Packages</Heading>
        <Flex>
          <PopularPackages />
        </Flex>
      </ContentSection>
    </>
  )
}
