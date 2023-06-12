import React, { useEffect, useState } from "react"
import styled from "styled-components"
import { ButtonLink } from "../../components/Button"
import CallToActionContainer from "../../components/CallToActionContainer"
import ContentSection from "../../components/ContentSection"
import PackageTag from "../../components/PackageTag"
import { Code, Heading, ResponsiveParagraph } from "../../components/Typography"
import { getWallyPackages, SearchResult } from "../../services/wally.api"

const Flex = styled.div`
  display: flex;
  flex-wrap: wrap;
  gap: 2rem;
  justify-content: space-around;
  margin: 1.5rem 0;
`

const PopularPackages = () => {
  const [count, setPackageCount] = useState(10);
  const [popularPackages, setPopularPackages] = useState<SearchResult[]>()
  const [isLoaded, setIsLoaded] = useState(false)

  const loadPopularPackages = async () => {
    const results = await getWallyPackages("*")
    setPopularPackages(results)
    setIsLoaded(true)
  }

  useEffect(() => {
    loadPopularPackages()
  }, [count])

  if (!isLoaded || popularPackages == undefined) {
    return (
      <>
        <ContentSection>
          <div>Loading popular packages...</div>
        </ContentSection>
      </>
    )
  }

  const popularPackagesList = popularPackages.slice(0, count).map(
    (popPackage, index) => (
      <PackageTag
        size="small"
        key={index}
        uniqueId={popPackage.name
          .substr(0, 15)
          .toLowerCase()
          .replace(/[^a-z]/gi, "")}
        scope={popPackage.scope}
        name={popPackage.name}
        version={popPackage.versions[0]}
        linkTo={`${popPackage.scope}/${popPackage.name}`}
      >
        <p>{popPackage.description}&nbsp;</p>
      </PackageTag>
    )
  )

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
          Wally is a package manager for Roblox inspired by Cargo (Rust) and npm
          (JavaScript). It brings the familiar, community-oriented world of
          sharing code from other communities into the Roblox ecosystem.
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
