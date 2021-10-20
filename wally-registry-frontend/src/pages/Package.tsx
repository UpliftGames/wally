import React from "react"
import { useParams } from "react-router"
import styled from "styled-components"
import { isMobile, notMobile } from "../breakpoints"
import ContentSection from "../components/ContentSection"
import CopyCode from "../components/CopyCode"
import { Heading, Paragraph } from "../components/Typography"
import mockPackages from "../mocks/packages.mock.js"

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
    padding-left: 2rem;
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
  const MetaItemWrapper = styled.div<StyledMetaItemProps>`
    width: ${(props) => (props.width === "full" ? "100%" : "50%")};
    display: inline-block;
    margin: 0.5rem 0;
  `

  return (
    <MetaItemWrapper width={width || "full"}>
      <MetaSubheader>{title}</MetaSubheader>
      {children}
    </MetaItemWrapper>
  )
}

type PackageParams = {
  packageSlug: string
}

export default function Package() {
  const { packageSlug } = useParams<PackageParams>()

  const packageData = mockPackages.find(
    (item) => item.package.name.split("/")[1] === packageSlug
  )

  return (
    <>
      <ContentSection>
        <FlexColumns>
          <WideColumn>
            <Heading>{packageSlug}</Heading>
            <Paragraph>
              <i>{packageData?.package.authors}</i>
            </Paragraph>

            <Paragraph>{packageData?.package.description}</Paragraph>
          </WideColumn>
          <NarrowColumn>
            <MetaHeader>Metadata</MetaHeader>

            <MetaItem title="Install" width="full">
              <CopyCode
                code={`${packageData?.package.name}@${packageData?.package.version}`}
              />
            </MetaItem>

            <MetaItem title="Version" width="half">
              {packageData?.package.version || "?.?.?"}
            </MetaItem>

            {packageData?.package.license && (
              <MetaItem title="License" width="half">
                {packageData?.package.license}
              </MetaItem>
            )}

            {packageData?.package.registry && (
              <MetaItem title="Repository" width="full">
                <a href={packageData?.package.registry}>
                  {packageData?.package.registry.replace("https://", "")}
                </a>
              </MetaItem>
            )}

            <MetaItem title="Authors" width="full">
              {packageData?.package.authors.map((author) => (
                <p key={author}>{author}</p>
              ))}
            </MetaItem>
          </NarrowColumn>
        </FlexColumns>
      </ContentSection>
    </>
  )
}
