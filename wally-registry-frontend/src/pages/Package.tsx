import React, { useEffect, useState } from "react"
import { useParams, useLocation, useHistory } from "react-router"
import styled from "styled-components"
import { isMobile, notMobile } from "../breakpoints"
import { Button } from "../components/Button"
import ContentSection from "../components/ContentSection"
import CopyCode from "../components/CopyCode"
import NotFoundMessage from "../components/NotFoundMessage"
import { Heading, Paragraph } from "../components/Typography"
import { getWallyPackageMetadata } from "../services/wally.api"
import { WallyPackageMetadata } from "../types/wally"
import capitalize from "../utils/capitalize"

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

const DependencyLink = ({ packageInfo }: { packageInfo: string }) => {
  let packageMatch = packageInfo.match(/(.+\/.+)@[^\d]+([\d\.]+)/)
  if (packageMatch != null) {
    let name = packageMatch[1]
    let version = packageMatch[2]
    return (
      <a
        href={`/package/${name}?version=${version}`}
        style={{ display: "block" }}
      >
        {name + "@" + version}
      </a>
    )
  }
  return (
    <a href={"/"} style={{ display: "block" }}>
      {packageInfo}
    </a>
  )
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
  const [packageVersion, setPackageVersion] = useState<string>()
  const [isLoaded, setIsLoaded] = useState(false)
  const [isError, setIsError] = useState(false)

  const urlPackageVersion = query.get("version")
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

  const packageMetadata = packageHistory?.find(
    (item: WallyPackageMetadata) => item.package.version === packageVersion
  )

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
                  {packageMetadata == undefined
                    ? `Couldn't find ${capitalize(
                        packageName
                      )} version ${packageVersion}. Are you sure that's a valid version?`
                    : packageMetadata?.package.description
                    ? packageMetadata?.package.description
                    : `${capitalize(packageName)} has no provided description.`}
                </Paragraph>
                {packageMetadata == undefined ? (
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
                ) : undefined}
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
                  <select
                    name="version"
                    id="version-select"
                    value={packageVersion || "?.?.?"}
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

                {packageMetadata?.dependencies &&
                  Object.values(packageMetadata?.dependencies).length > 0 && (
                    <MetaItem title="Dependencies" width="full">
                      {Object.values(packageMetadata?.dependencies).map(
                        (dependency) => (
                          <DependencyLink
                            key={dependency}
                            packageInfo={dependency}
                          />
                        )
                      )}
                    </MetaItem>
                  )}

                {packageMetadata &&
                  packageMetadata["server-dependencies"] &&
                  Object.values(packageMetadata["server-dependencies"]).length >
                    0 && (
                    <MetaItem title="Server Dependencies" width="full">
                      {Object.values(
                        packageMetadata["server-dependencies"]
                      ).map((dependency) => (
                        <DependencyLink
                          key={dependency}
                          packageInfo={dependency}
                        />
                      ))}
                    </MetaItem>
                  )}

                {packageMetadata &&
                  packageMetadata["dev-dependencies"] &&
                  Object.values(packageMetadata["dev-dependencies"]).length >
                    0 && (
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
