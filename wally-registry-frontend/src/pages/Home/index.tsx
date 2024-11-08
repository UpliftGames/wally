import React, { useState, useEffect } from "react"
import styled from "styled-components"
import { ButtonLink } from "../../components/Button"
import CallToActionContainer from "../../components/CallToActionContainer"
import ContentSection from "../../components/ContentSection"
import PackageTag from "../../components/PackageTag"
import { Code, Heading, ResponsiveParagraph } from "../../components/Typography"
import mockPopularPackages from "../../mocks/popularPackages.mock"
import { getWallyPackageMetadata } from "../../services/wally.api"
import { WallyPackageMetadata } from "../../types/wally"
import capitalize from "../../utils/capitalize"

const Flex = styled.div`
  display: flex;
  flex-wrap: wrap;
  gap: 2rem;
  justify-content: space-around;
  margin: 1.5rem 0;
`

const SplitPackageFullName = (
  fullName: string
): { Scope: string; Name: string } => {
  const [scope, name] = fullName.split("/")
  return { Scope: scope, Name: name }
}

const PopularPackages = () => {
  const [popularPackages, setPopularPackages] = useState<
    WallyPackageMetadata[]
  >([])

  const fetchPopularPackages = async () => {
    const popularPackagesList = []

    for (const fullName of [...mockPopularPackages]) {
      const { Scope, Name } = SplitPackageFullName(fullName)
      const packageData = await getWallyPackageMetadata(Scope, Name)
      if (!packageData) continue

      // boatbomber src/pages/Package.tsx is an excerpt from the code snippet he wrote
      const filteredPackageData = packageData.versions.some(
        (pack: WallyPackageMetadata) => !pack.package.version.includes("-")
      )
        ? packageData.versions.filter(
            (pack: WallyPackageMetadata) => !pack.package.version.includes("-")
          )
        : packageData.versions

      const latestVersion = filteredPackageData[0].package.version
      const packageMetadata = filteredPackageData?.find(
        (item: WallyPackageMetadata) => item.package.version === latestVersion
      )

      popularPackagesList.push(packageMetadata)
    }

    setPopularPackages(popularPackagesList)
  }

  useEffect(() => {
    fetchPopularPackages()
  }, [])

  return (
    <>
      {popularPackages.map((popPackage, index) => (
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
          <p>
            {popPackage.package.description ??
              `${capitalize(
                SplitPackageFullName(popPackage.package.name).Name
              )} has no provided description.`}
            &nbsp;
          </p>
        </PackageTag>
      ))}
    </>
  )
}

export default function Home() {
  return (
    <>
      <ContentSection variation="red" placement="floating">
        <Heading>Wally, a package manager for Roblox</Heading>
        <ResponsiveParagraph>
          Wally is a package manager for Roblox inspired by Cargo (Rust) and npm
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
