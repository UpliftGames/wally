import React, { useEffect, useState } from "react"
import { useLocation } from "react-router-dom"
import styled from "styled-components"
import { isMobile } from "../breakpoints"
import ContentSection from "../components/ContentSection"
import PackageTag from "../components/PackageTag"
import { Code, Heading } from "../components/Typography"
import { getWallyPackages } from "../services/wally.api"
import { WallyPackageBrief } from "../types/wally"

function useQuery() {
  return new URLSearchParams(useLocation().search)
}

const Flex = styled.div`
  display: flex;
  flex-wrap: wrap;
  gap: 2rem;
  justify-content: space-around;
  margin: 1.5rem 0;
  padding: 0 1rem;

  @media screen and (${isMobile}) {
    padding: 0;
  }
`

const SearchPackages = ({ packages }: { packages: WallyPackageBrief[] }) => (
  <>
    {[...packages].map((pack: WallyPackageBrief, index) => (
      <PackageTag
        size="large"
        width="wide"
        key={index}
        uniqueId={pack.name
          .substr(0, 15)
          .toLowerCase()
          .replace(/[^a-z]/gi, "")}
        scope={pack.scope}
        name={pack.name}
        version={pack.versions.at(-1)}
        linkTo={`${pack.scope}/${pack.name}`}
      >
        <p>{pack.description}&nbsp;</p>
      </PackageTag>
    ))}
  </>
)

export default function Search() {
  const queryParams = useQuery()
  const searchQuery = queryParams.get("q")
  const [packages, setPackages] = useState<WallyPackageBrief[]>()
  const [isLoaded, setIsLoaded] = useState(false)
  const [isError, setIsError] = useState(false)

  const loadPackagesListData = async (searchQuery: string) => {
    const packagesListData = await getWallyPackages(searchQuery)
    if (packagesListData === undefined || packagesListData.length === 0) {
      setIsError(true)
      setIsLoaded(true)
    } else {
      setPackages(packagesListData)
      setIsLoaded(true)
    }
  }

  useEffect(() => {
    if (searchQuery && !isLoaded && !isError) {
      loadPackagesListData(searchQuery)
    }
  }, [searchQuery])

  return (
    <>
      <ContentSection variation="red">
        <Heading>
          Results for: <Code>{searchQuery}</Code>
        </Heading>
      </ContentSection>

      <ContentSection variation="light">
        <Flex>
          {isLoaded ? (
            isError ? (
              <p>No packages found</p>
            ) : (
              packages && <SearchPackages packages={packages} />
            )
          ) : (
            <div>Loading...</div>
          )}
        </Flex>
      </ContentSection>
    </>
  )
}
