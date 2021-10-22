import React, { useEffect, useState } from "react"
import { useLocation } from "react-router-dom"
import styled from "styled-components"
import ContentSection from "../components/ContentSection"
import PackageTag from "../components/PackageTag"
import { Code, Heading } from "../components/Typography"
import { getWallyPackages } from "../services/wally.api"

type PackageBrief = {
  description: string
  name: string
  scope: string
  version: string
}

function useQuery() {
  return new URLSearchParams(useLocation().search)
}

const Flex = styled.div`
  display: flex;
  flex-wrap: wrap;
  gap: 2rem;
  justify-content: space-around;
  margin: 1.5rem 0;
`

const SearchPackages = ({ packages }: { packages: PackageBrief[] }) => (
  <>
    {[...packages].map((pack: PackageBrief, index) => (
      <PackageTag
        size="large"
        width="wide"
        key={index}
        uniqueId={pack.name
          .substr(0, 15)
          .toLowerCase()
          .replace(/[^a-z]/gi, "")}
        title={`${pack.scope}/${pack.name}`}
        version={pack.version}
        linkTo={`${pack.scope}/${pack.name}`}
      >
        <p>{pack.description} &nbsp;</p>
      </PackageTag>
    ))}
  </>
)

export default function Search() {
  const queryParams = useQuery()
  const searchQuery = queryParams.get("q")
  const [packages, setPackages] = useState<PackageBrief[]>()
  const [isLoaded, setIsLoaded] = useState(false)

  const loadPackagesListData = async (searchQuery: string) => {
    const packagesListData = await getWallyPackages(searchQuery)
    setPackages(packagesListData)
  }

  useEffect(() => {
    if (!isLoaded && searchQuery) {
      loadPackagesListData(searchQuery)
      setIsLoaded(true)
    }
  }, [])

  return (
    <>
      <ContentSection variation="red">
        <div>
          <Heading>
            Results for: <Code>{searchQuery}</Code>
          </Heading>
        </div>
      </ContentSection>

      <ContentSection variation="light">
        {/* <pre style={{ wordBreak: "break-all", whiteSpace: "pre-line" }}>
          <code>{packages}</code>
        </pre> */}
        <Flex>
          {packages ? (
            <SearchPackages packages={packages} />
          ) : (
            <div>Loading...</div>
          )}
        </Flex>
      </ContentSection>
    </>
  )
}
