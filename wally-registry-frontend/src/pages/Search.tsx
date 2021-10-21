import React from "react"
import { useLocation } from "react-router-dom"
import styled from "styled-components"
import ContentSection from "../components/ContentSection"
import PackageTag from "../components/PackageTag"
import { Code, Heading } from "../components/Typography"
import mockPackages from "../mocks/packages.mock"

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

const SearchPackages = () => (
  <>
    {[...mockPackages].map((popPackage, index) => (
      <PackageTag
        size="large"
        width="wide"
        key={index}
        uniqueId={popPackage.package?.name
          .substr(0, 15)
          .toLowerCase()
          .replace(/[^a-z]/gi, "")}
        title={popPackage.package.name}
        author={popPackage.package.authors.join(" ")}
        version={popPackage.package.version}
        linkTo={popPackage.package.name.split("/")[1]}
      >
        <p>{popPackage.package.description}</p>
      </PackageTag>
    ))}
  </>
)

export default function Search() {
  const queryParams = useQuery()

  return (
    <>
      <ContentSection variation="red">
        <div>
          <Heading>
            Results for: <Code>{queryParams.get("q")}</Code>
          </Heading>
        </div>
      </ContentSection>

      <ContentSection variation="light">
        <Flex>
          <SearchPackages />
        </Flex>
      </ContentSection>
    </>
  )
}
