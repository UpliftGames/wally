"use client"

import { ButtonLink } from "@/components/Button"
import CallToActionContainer from "@/components/CallToActionContainer"
import ContentSection from "@/components/ContentSection"
import PackageTag from "@/components/PackageTag"
import { Code, Heading, ResponsiveParagraph } from "@/components/Typography"
import styled from "styled-components"
import mockPopularPackages from "../mocks/popularPackages.mock"

const Flex = styled.div`
  display: flex;
  flex-wrap: wrap;
  gap: 2rem;
  justify-content: space-around;
  margin: 1.5rem 0;
`

const BannerInlineLink = styled.a`
  border-bottom: 1px dotted var(--wally-white);

  &:hover,
  &:focus {
    color: var(--wally-mauve);
    border-bottom: 1px solid var(--wally-mauve);
    cursor: help;
  }
`

const PopularPackages = () => {
  const popularPackagesList = mockPopularPackages.map((popPackage, index) => (
    <PackageTag
      size="small"
      key={index}
      uniqueId={popPackage.package.name
        .substr(0, 15)
        .toLowerCase()
        .replace(/[^a-z]/gi, "")}
      title={popPackage.package.name}
      author={popPackage.package.authors.join(" ")}
      version={popPackage.package.version}
      linkTo={popPackage.package.name}
    >
      <p>{popPackage.package.description}&nbsp;</p>
    </PackageTag>
  ))

  if (popularPackagesList.length % 2 !== 0) {
    popularPackagesList.push(<PackageTag size="small" hidden={true} />)
  }

  return <>{popularPackagesList}</>
}

export default function Home() {
  return (
    <>
      <ContentSection variation="red" placement="floating">
        <Heading>Wally, a package manager for Roblox</Heading>
        <ResponsiveParagraph>
          Wally is a package manager for Roblox inspired by{" "}
          <BannerInlineLink
            href="https://doc.rust-lang.org/stable/cargo/"
            target="_blank"
            rel="noopener"
          >
            Cargo
          </BannerInlineLink>{" "}
          (Rust) and{" "}
          <BannerInlineLink
            href="https://www.npmjs.com/"
            target="_blank"
            rel="noopener"
          >
            npm
          </BannerInlineLink>{" "}
          (JavaScript). It brings the familiar, community-oriented world of
          sharing code from other communities into the Roblox ecosystem.
        </ResponsiveParagraph>
        <ResponsiveParagraph>
          Wally has two pieces that work together: a command line tool named{" "}
          <Code>wally</Code> and a registry server that hosts packages. Most
          users will only interact with the command line tool, but both are
          available in this repository.
        </ResponsiveParagraph>

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
