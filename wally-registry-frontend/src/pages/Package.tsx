import React, { useEffect, useState } from "react"
import { useParams } from "react-router"
import styled from "styled-components"
import { isMobile, notMobile } from "../breakpoints"
import ContentSection from "../components/ContentSection"
import CopyCode from "../components/CopyCode"
import NotFoundMessage from "../components/NotFoundMessage"
import { Heading, Paragraph } from "../components/Typography"
import { getWallyPackageMetadata } from "../services/wally.api"
import { WallyPackageMetadata } from "../types/wally"
import capitalize from "../utils/capitalize"

type WidthVariation = "full" | "half"

interface StyledMetaItemProps {
  width: WidthVariation
}

const FlexColumns = styled.article`
  display: flex;
  flex-flow: row nowrap;
  width: 100%;
  min-height: 65vh;

  @media screen and (${isMobile}) {
    flex-flow: row wrap;
  }
`

const WideColumn = styled.section`
  width: 70%;

  @media screen and (${notMobile}) {
    border-right: solid 2px rgba(0, 0, 0, 0.1);
  }

  @media screen and (${isMobile}) {
    width: 100%;
    border-bottom: solid 2px rgba(0, 0, 0, 0.1);
  }
`

const NarrowColumn = styled.aside`
  width: 30%;

  @media screen and (${notMobile}) {
    padding-left: 1rem;
  }

  @media screen and (${isMobile}) {
    padding-top: 0.5rem;
    width: 100%;
  }
`

const MetaHeader = styled.h2`
  width: 100%;
  font-size: 2rem;
`

const MetaSubheader = styled.b`
  font-weight: bold;
  display: block;
  font-size: 1.1rem;
`

const MetaItemWrapper = styled.div<StyledMetaItemProps>`
  width: ${(props) => (props.width === "full" ? "100%" : "50%")};
  display: inline-block;
  margin: 0.5rem 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;

  a:hover,
  a:focus {
    text-decoration: underline;
    color: var(--wally-red);
  }
`

const MetaItem = ({
  title,
  width,
  children,
}: {
  title: string
  width?: WidthVariation
  children: React.ReactNode
}) => {
  return (
    <MetaItemWrapper width={width || "full"}>
      <MetaSubheader>{title}</MetaSubheader>
      {children}
    </MetaItemWrapper>
  )
}

type PackageParams = {
  packageScope: string
  packageName: string
}

export default function Package() {
  const { packageScope, packageName } = useParams<PackageParams>()
  const [packageMetadata, setPackageMetadata] = useState<WallyPackageMetadata>()
  const [isLoaded, setIsLoaded] = useState(false)
  const [isError, setIsError] = useState(false)

  const loadPackageData = async (packageScope: string, packageName: string) => {
    const packageData = await getWallyPackageMetadata(packageScope, packageName)
    if (packageData !== undefined) {
      const filteredPackageData = packageData.versions.some(
        (pack: WallyPackageMetadata) => !pack.package.version.includes("-")
      )
        ? packageData.versions.filter(
            (pack: WallyPackageMetadata) => !pack.package.version.includes("-")
          )
        : packageData
      setPackageMetadata(filteredPackageData[0])
      setIsLoaded(true)
    } else {
      setIsError(true)
      setIsLoaded(true)
    }
  }

  useEffect(() => {
    loadPackageData(packageScope, packageName)
  }, [packageScope, packageName])

  return (
    <>
      <ContentSection>
        {isLoaded ? (
          isError ? (
            <NotFoundMessage errorMessage="HTTP 404: Resource Not Found" />
          ) : (
            <FlexColumns>
              <WideColumn>
                <Heading>{packageName}</Heading>

                <Paragraph>
                  {packageMetadata?.package.description
                    ? packageMetadata?.package.description
                    : `${capitalize(
                        packageMetadata?.package.name
                      )} has no provided description.`}
                </Paragraph>
              </WideColumn>
              <NarrowColumn>
                <MetaHeader>Metadata</MetaHeader>

                {packageMetadata?.package && (
                  <MetaItem title="Install" width="full">
                    <CopyCode
                      packageName={packageMetadata?.package.name}
                      version={packageMetadata?.package.version}
                    />
                  </MetaItem>
                )}

                <MetaItem title="Version" width="half">
                  {packageMetadata?.package.version || "?.?.?"}
                </MetaItem>

                {packageMetadata?.package.license && (
                  <MetaItem title="License" width="half">
                    <a
                      href={`https://choosealicense.com/licenses/${packageMetadata?.package.license.toLocaleLowerCase()}`}
                    >
                      {packageMetadata?.package.license}
                    </a>
                  </MetaItem>
                )}

                {packageMetadata?.package.realm && (
                  <MetaItem title="Realm" width="half">
                    {capitalize(packageMetadata?.package.realm)}
                  </MetaItem>
                )}

                {/* TODO: Re-implement when Wally API supports custom source repos */}
                {/* {packageMetadata?.package.registry && (
                <MetaItem title="Repository" width="full">
                  <a href={packageMetadata?.package.registry}>
                    {packageMetadata?.package.registry.replace("https://", "")}
                  </a>
                </MetaItem>
              )} */}

                {packageMetadata?.package.authors &&
                  packageMetadata?.package.authors.length > 0 && (
                    <MetaItem title="Authors" width="full">
                      {packageMetadata?.package.authors.map((author) => (
                        <p key={author}>{author}</p>
                      ))}
                    </MetaItem>
                  )}
              </NarrowColumn>
            </FlexColumns>
          )
        ) : (
          <div>Loading...</div>
        )}
      </ContentSection>
    </>
  )
}
