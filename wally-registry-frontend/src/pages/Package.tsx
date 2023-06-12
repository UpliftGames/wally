/* eslint-disable react/no-unescaped-entities */
import React, { useEffect, useState } from "react"
import { useHistory, useLocation, useParams } from "react-router"
import styled from "styled-components"
import { isMobile, notMobile } from "../breakpoints"
import { Button } from "../components/Button"
import ContentSection from "../components/ContentSection"
import CopyCode from "../components/CopyCode"
import NotFoundMessage from "../components/NotFoundMessage"
import { Heading, Paragraph } from "../components/Typography"
import { getWallyPackageMetadata, getWallyPackageAnalytics } from "../services/wally.api"
import { WallyPackageMetadata, WallyPackageAnalytics } from "../types/wally"
import capitalize from "../utils/capitalize"
import {
  Accordion,
  AccordionItem,
  AccordionItemHeading,
  AccordionItemButton,
  AccordionItemPanel,
} from 'react-accessible-accordion';

// A custom hook that builds on useLocation to parse
// the query string for you.
function useQuery() {
  const { search } = useLocation()

  return React.useMemo(() => new URLSearchParams(search), [search])
}

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
  width: 65%;

  @media screen and (${notMobile}) {
    border-right: solid 2px rgba(0, 0, 0, 0.1);
  }

  @media screen and (${isMobile}) {
    width: 100%;
    border-bottom: solid 2px rgba(0, 0, 0, 0.1);
  }
`

const NarrowColumn = styled.aside`
  width: 35%;

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
  text-overflow: ellipsis;

  a:hover,
  a:focus {
    text-decoration: underline;
    color: var(--wally-red);
  }
`

const AuthorItem = styled.p`
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
`

const DependencyLinkWrapper = styled.div`
  display: flex;
  flex-wrap: wrap;
  position: relative;
  width: 100%;

  &:hover {
    > span {
      visibility: visible;
    }
  }
`

const DependencyLinkItem = styled.a`
  display: block;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
`

const DependencyVersionReq = styled.a`
  display: flex;
  margin-left: auto;
  text-align: right;
  flext-wrap: wrap;
  opacity: 0.6;
`

const DependencyLinkSpan = styled.span`
  width:100%;
  height:1em;
  display:inline-block;
`

const DependencyLinkTooltip = styled.span`
  visibility: hidden;
  position: absolute;
  z-index: 2;
  color: white;
  font-size: 0.8rem;
  background-color: var(--wally-brown);
  border-radius: 5px;
  padding: 10px;
  top: -45px;
  left: 50%;
  transform: translateX(-50%);

  &::before {
    content: "";
    position: absolute;
    transform: rotate(45deg);
    background-color: var(--wally-brown);
    padding: 6px;
    z-index: 1;
    top: 77%;
    left: 45%;
  }
`

const AccordionCSS = `.accordion {
}

.accordion__item + .accordion__item {
  border-top: 1px solid rgba(0, 0, 0, 0.1);
}

.accordion__button {
  font-weight: bold;
  display: block;
  font-size: 1.1rem;
  margin: 0.5rem 0;
  display: block;
  background-color: #f4f4f4;
  color: #444;
  cursor: pointer;
  width: 100%;
  text-align: left;
  border: none;
}

.accordion__button:hover {
  background-color: #ddd;
}

.accordion__button:after {
  display: inline-block;
  content: '';
  height: 10px;
  width: 10px;
  margin-left: 12px;
  border-bottom: 2px solid currentColor;
  border-right: 2px solid currentColor;
  transform: rotate(-45deg);
}

.accordion__button[aria-expanded='true']::after,
.accordion__button[aria-selected='true']::after {
  transform:  translate(0, -4px) rotate(45deg);
}

[hidden] {
  display: none;
}

.accordion__panel {
  animation: fadein 0.15s ease-in;
}

@keyframes fadein {
  0% {
      opacity: 0;
  }

  100% {
      opacity: 1;
  }
}`

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

const DependencyLink = ({ packageInfo }: { packageInfo: string }) => {
  let packageMatch = packageInfo.match(/(.+\/.+)@[^\d]+([\d.]+)/)
  if (packageMatch != null) {
    let name = packageMatch[1]
    let version = packageMatch[2]
    return (
      <DependencyLinkWrapper>
        <DependencyLinkItem href={`/package/${name}?version=${version}`}>
          {name + "@" + version}
        </DependencyLinkItem>
        <DependencyLinkTooltip>{name + "@" + version}</DependencyLinkTooltip>
      </DependencyLinkWrapper>
    )
  }
  return <DependencyLinkItem href={"/"}>{packageInfo}</DependencyLinkItem>
}

const LatestDependencyLink = ({ packageInfo, versionReq }: { packageInfo: string, versionReq: string }) => {
  let packageMatch = packageInfo.match(/(.+\/.+)/)
  if (packageMatch != null) {
    let name = packageMatch[1]
    return (
      <DependencyLinkWrapper>
        <DependencyLinkItem href={`/package/${name}`}>
          {name}
        </DependencyLinkItem>
        <DependencyVersionReq>{versionReq}</DependencyVersionReq>
        <DependencyLinkTooltip>{name}</DependencyLinkTooltip>
      </DependencyLinkWrapper>
    )
  }
  return <DependencyLinkItem href={"/"}>{packageInfo}</DependencyLinkItem>
}

type PackageParams = {
  packageScope: string
  packageName: string
}

export default function Package() {
  const query = useQuery()
  const hist = useHistory()

  const { packageScope, packageName } = useParams<PackageParams>()
  const [packageHistory, setPackageHistory] = useState<[WallyPackageMetadata]>()
  const [packageAnalytics, setPackageAnalytics] = useState<WallyPackageAnalytics>()
  const [packageVersion, setPackageVersion] = useState<string>()
  const [isLoaded, setIsLoaded] = useState(false)
  const [isError, setIsError] = useState(false)

  const urlPackageVersion = query.get("version")
  if (urlPackageVersion != null && urlPackageVersion !== packageVersion) {
    setPackageVersion(urlPackageVersion)
  }

  const loadPackageData = async (packageScope: string, packageName: string) => {
    const packageData = await getWallyPackageMetadata(packageScope, packageName)
    const packageAnalytics = await getWallyPackageAnalytics(packageScope, packageName)

    if (packageData == undefined) {
      setIsError(true)
      setIsLoaded(true)
      return
    }

    // If analytics fails we should just quietly hide them rather than abort the whole page
    if (packageAnalytics != undefined) {
      setPackageAnalytics(packageAnalytics)
    }

    const filteredPackageData = packageData.versions.some(
      (pack: WallyPackageMetadata) => !pack.package.version.includes("-")
    )
      ? packageData.versions.filter(
        (pack: WallyPackageMetadata) => !pack.package.version.includes("-")
      )
      : packageData

    setPackageHistory(filteredPackageData)

    if (urlPackageVersion == null) {
      const latestVersion = filteredPackageData[0].package.version
      setPackageVersion(latestVersion)
      hist.replace(
        `/package/${packageScope}/${packageName}?version=${latestVersion}`
      )
    }

    setIsLoaded(true)
  }

  useEffect(() => {
    loadPackageData(packageScope, packageName)
  }, [packageScope, packageName])

  if (!isLoaded) {
    return (
      <>
        <ContentSection>
          <div>Loading...</div>
        </ContentSection>
      </>
    )
  }

  if (isError) {
    return (
      <>
        <ContentSection>
          <NotFoundMessage errorMessage="HTTP 404: Resource Not Found" />
        </ContentSection>
      </>
    )
  }

  const packageMetadata = packageHistory?.find(
    (item: WallyPackageMetadata) => item.package.version === packageVersion
  )

  if (packageMetadata == undefined) {
    return (
      <>
        <ContentSection>
          <Heading>{packageName}</Heading>

          <Paragraph>
            Couldn't find {capitalize(packageName)} version {packageVersion}.
            Are you sure that's a valid version?
          </Paragraph>

          <Button
            onClick={() => {
              if (packageHistory == undefined) {
                return
              }
              hist.push(
                `/package/${packageScope}/${packageName}?version=${packageHistory[0].package.version}`
              )
            }}
          >
            View Latest Version
          </Button>
        </ContentSection>
      </>
    )
  }

  return (
    <>
      <ContentSection>
        <FlexColumns>
          <WideColumn>
            <Heading>{packageName}</Heading>

            <Paragraph>
              {packageMetadata.package.description ?? `${capitalize(packageName)} has no provided description.`}
            </Paragraph>
          </WideColumn>

          <NarrowColumn>
            <MetaHeader>Metadata</MetaHeader>

            <MetaItem title="Install" width="full">
              <CopyCode
                packageName={packageMetadata.package.name}
                version={packageMetadata.package.version}
              />
            </MetaItem>

            <MetaItem title="Version" width="half">
              <select
                name="version"
                id="version-select"
                value={packageVersion ?? "?.?.?"}
                onChange={(a) => {
                  hist.push(
                    `/package/${packageScope}/${packageName}?version=${a.target.value}`
                  )
                }}
              >
                {packageHistory?.map((item: WallyPackageMetadata) => {
                  return (
                    <option
                      key={item.package.version}
                      value={item.package.version}
                    >
                      {item.package.version}
                    </option>
                  )
                })}
              </select>
            </MetaItem>

            {packageMetadata.package.license && (
              <MetaItem title="License" width="half">
                <a
                  href={`https://choosealicense.com/licenses/${packageMetadata?.package.license.toLocaleLowerCase()}`}
                >
                  {packageMetadata?.package.license}
                </a>
              </MetaItem>
            )}

            <MetaItem title="Realm" width="half">
              {capitalize(packageMetadata.package.realm)}
            </MetaItem>

            {/* TODO: Re-implement when Wally API supports custom source repos */}
            {/* {packageMetadata?.package.registry && (
                <MetaItem title="Repository" width="full">
                  <a href={packageMetadata?.package.registry}>
                    {packageMetadata?.package.registry.replace("https://", "")}
                  </a>
                </MetaItem>
              )} */}

            {packageMetadata.package.authors.length > 0 && (
              <MetaItem title="Authors" width="full">
                {packageMetadata.package.authors.map((author) => (
                  <AuthorItem key={author}>{author}</AuthorItem>
                ))}
              </MetaItem>
            )}

            {Object.keys(packageMetadata.dependencies).length > 0 && (
              <MetaItem title="Dependencies" width="full">
                {Object.values(packageMetadata.dependencies).map(
                  (dependency) => (
                    <DependencyLink
                      key={dependency}
                      packageInfo={dependency}
                    />
                  )
                )}
              </MetaItem>
            )}

            {Object.keys(packageMetadata["server-dependencies"]).length > 0 && (
              <MetaItem title="Server Dependencies" width="full">
                {Object.values(packageMetadata["server-dependencies"]).map(
                  (dependency) => (
                    <DependencyLink
                      key={dependency}
                      packageInfo={dependency}
                    />
                  )
                )}
              </MetaItem>
            )}

            {Object.keys(packageMetadata["dev-dependencies"]).length > 0 && (
              <MetaItem title="Dev Dependencies" width="full">
                {Object.values(packageMetadata["dev-dependencies"]).map(
                  (dependency) => (
                    <DependencyLink
                      key={dependency}
                      packageInfo={dependency}
                    />
                  )
                )}
              </MetaItem>
            )}

            {packageAnalytics != undefined && (
              <Accordion allowZeroExpanded>
                <style>
                  {AccordionCSS}
                </style>
                <AccordionItem>
                  <AccordionItemHeading>
                    <AccordionItemButton>
                      {`${packageAnalytics.dependents.length} Dependents`}
                    </AccordionItemButton>
                  </AccordionItemHeading>
                  <AccordionItemPanel>
                    {Object.values(packageAnalytics.dependents).map(
                      (dependency) => (
                        <LatestDependencyLink
                          key={dependency[0]}
                          packageInfo={dependency[0]}
                          versionReq={dependency[1]}
                        />
                      )
                    )}
                  </AccordionItemPanel>
                </AccordionItem>
              </Accordion>
            )}
          </NarrowColumn>
        </FlexColumns>
      </ContentSection>
    </>
  )
}
