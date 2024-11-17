"use client"

import { isMobile, notMobile } from "@/breakpoints"
import { Button } from "@/components/Button"
import ContentSection from "@/components/ContentSection"
import CopyCode from "@/components/CopyCode"
import NotFoundMessage from "@/components/NotFoundMessage"
import { Heading, Paragraph } from "@/components/Typography"
import {
  buildWallyPackageDownloadLink,
  getWallyPackageMetadata,
} from "@/services/wally.api"
import { WallyPackageMetadata } from "@/types/wally"
import capitalize from "@/utils/capitalize"
import { useParams, useRouter, useSearchParams } from "next/navigation"
import React, { useEffect, useState } from "react"
import styled from "styled-components"

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

const VersionSelect = styled.select`
  &:hover {
    cursor: pointer;
  }

  &:hover,
  &:focus {
    text-decoration: underline;
    color: var(--wally-red);
  }

  > option {
    color: var(--wally-mauve);
  }
`

const AuthorItem = styled.p`
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
`

const DependencyLinkWrapper = styled.div`
  display: block;
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

const StyledDownloadLink = styled.a`
  display: inline-block;
  height: 1rem;
  width: auto;
  padding-right: 0.3rem;
  cursor: pointer;
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

const DependencyLink = ({ packageInfo }: { packageInfo: string }) => {
  const packageMatch = packageInfo.match(/(.+\/.+)@[^\d]+([\d.]+)/)
  if (packageMatch != null) {
    const name = packageMatch[1]
    const version = packageMatch[2]
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

const MetadataLink = ({
  url
}: {
  url: string

}) => {
  const Link = styled.a`
    overflow: hidden;
    text-overflow: ellipsis;
    display: flex;
    white-space: nowrap;
    width: 100%;
    display: inline-block;
    font-size: 18px;
  `

  const urlNoSchema = (original: string) => {
    try {
      const url = new URL(original)
      return `${url.host}${url?.pathname}`
    } catch {
      return original
    }
  }

  return (
    <>
      <Link href={url} title={url}>
        {urlNoSchema(url)}
      </Link>
    </>
  )
}

const DownloadLink = ({
  url,
  filename,
  children,
}: {
  url: string
  filename: string
  children: React.ReactNode
}) => {
  const handleAction = async () => {
    // Using bare JavaScript mutations over a React ref keeps this link tag keyboard accessible, because you can't include an href on the base anchor tag and overwrite it with a ref, and we need an href to ensure full keyboard compatibility
    const link = document.createElement("a")

    const result = await fetch(url, {
      headers: {
        "wally-version": "0.3.2",
      },
    })

    const blob = await result.blob()
    const href = window.URL.createObjectURL(blob)

    link.href = href
    link.download = filename

    document.body.appendChild(link)

    link.click()

    link.parentNode?.removeChild(link)
  }

  return (
    <>
      <StyledDownloadLink role="button" href="#" onClick={handleAction}>
        {children}
      </StyledDownloadLink>
    </>
  )
}

const DownloadIcon = ({ packageName }: { packageName: string }) => (
  <svg
    role="img"
    aria-label="[title + description]"
    xmlns="http://www.w3.org/2000/svg"
    width="512"
    height="512"
    style={{ height: "100%", width: "100%" }}
    enableBackground="new 0 0 512 512"
    viewBox="0 0 512 512"
  >
    <title>Download a Wally Package</title>
    <desc>Downloads {packageName}</desc>
    <g fill="currentColor">
      <path d="M382.56 233.376A15.96 15.96 0 00368 224h-64V16c0-8.832-7.168-16-16-16h-64c-8.832 0-16 7.168-16 16v208h-64a16.013 16.013 0 00-14.56 9.376c-2.624 5.728-1.6 12.416 2.528 17.152l112 128A15.946 15.946 0 00256 384c4.608 0 8.992-2.016 12.032-5.472l112-128c4.16-4.704 5.12-11.424 2.528-17.152z"></path>
      <path d="M432 352v96H80v-96H16v128c0 17.696 14.336 32 32 32h416c17.696 0 32-14.304 32-32V352h-64z"></path>
    </g>
  </svg>
)

type PackageParams = {
  packageScope: string
  packageName: string
}

export const runtime = "edge";

export default function Package() {
  const searchParams = useSearchParams()
  const router = useRouter()

  const { packageScope, packageName } = useParams<PackageParams>()
  const [packageHistory, setPackageHistory] = useState<[WallyPackageMetadata]>()
  const [packageVersion, setPackageVersion] = useState<string>()
  const [isLoaded, setIsLoaded] = useState(false)
  const [isError, setIsError] = useState(false)

  const urlPackageVersion = searchParams.get("version")
  if (urlPackageVersion != null && urlPackageVersion !== packageVersion) {
    setPackageVersion(urlPackageVersion)
  }

  const loadPackageData = async (packageScope: string, packageName: string) => {
    const packageData = await getWallyPackageMetadata(packageScope, packageName)

    if (packageData == undefined) {
      setIsError(true)
      setIsLoaded(true)
      return
    }

    const filteredPackageData = packageData.versions.some(
      (pack: WallyPackageMetadata) => !pack.package.version.includes("-")
    )
      ? packageData.versions.filter(
        (pack: WallyPackageMetadata) => !pack.package.version.includes("-")
      )
      : packageData.versions

    setPackageHistory(filteredPackageData)

    if (urlPackageVersion == null) {
      const latestVersion = filteredPackageData[0].package.version
      setPackageVersion(latestVersion)
      router.replace(
        `/package/${packageScope}/${packageName}?version=${latestVersion}`
      )
    }

    setIsLoaded(true)
  }

  useEffect(() => {
    loadPackageData(packageScope, packageName)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [packageScope, packageName])

  if (!isLoaded) {
    return (
      <>
        <ContentSection>
          <FlexColumns>Loading...</FlexColumns>
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
            Couldn&apos;t find {capitalize(packageName)} version{" "}
            {packageVersion}. Are you sure that&apos;s a valid version?
          </Paragraph>

          <Button
            onClick={() => {
              if (packageHistory == undefined) {
                return
              }
              router.push(
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
              {packageMetadata.package.description ??
                `${capitalize(packageName)} has no provided description.`}
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
              <VersionSelect
                name="version"
                id="version-select"
                value={packageVersion ?? "?.?.?"}
                onChange={(a) => {
                  router.push(
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
              </VersionSelect>
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

            <MetaItem title="Download" width="half">
              <DownloadLink
                url={buildWallyPackageDownloadLink(
                  packageScope,
                  packageName,
                  packageMetadata.package.version
                )}
                filename={
                  packageScope +
                  "/" +
                  packageName +
                  "@" +
                  packageMetadata.package.version +
                  ".zip"
                }
              >
                <DownloadIcon packageName={packageScope + "/" + packageName} />
              </DownloadLink>
            </MetaItem>

            <MetaItem title="Realm" width="half">
              {capitalize(packageMetadata.package.realm)}
            </MetaItem>

            {packageMetadata?.package.homepage && (
              <MetaItem title="Homepage" width="full">
                <MetadataLink url={packageMetadata?.package.homepage} />
              </MetaItem>
            )}

            {packageMetadata?.package.repository && (
              <MetaItem title="Repository" width="full">
                <MetadataLink url={packageMetadata?.package.repository} />
              </MetaItem>
            )}

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
                    <DependencyLink key={dependency} packageInfo={dependency} />
                  )
                )}
              </MetaItem>
            )}

            {Object.keys(packageMetadata["server-dependencies"]).length > 0 && (
              <MetaItem title="Server Dependencies" width="full">
                {Object.values(packageMetadata["server-dependencies"]).map(
                  (dependency) => (
                    <DependencyLink key={dependency} packageInfo={dependency} />
                  )
                )}
              </MetaItem>
            )}

            {Object.keys(packageMetadata["dev-dependencies"]).length > 0 && (
              <MetaItem title="Dev Dependencies" width="full">
                {Object.values(packageMetadata["dev-dependencies"]).map(
                  (dependency) => (
                    <DependencyLink key={dependency} packageInfo={dependency} />
                  )
                )}
              </MetaItem>
            )}
          </NarrowColumn>
        </FlexColumns>
      </ContentSection>
    </>
  )
}
