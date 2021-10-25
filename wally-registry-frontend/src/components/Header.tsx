import React, { useState } from "react"
import { useHistory } from "react-router"
import { Link, NavLink } from "react-router-dom"
import styled from "styled-components"
import logo from "../assets/wally-logo.svg"
import { isCondensed, isMobile, notMobile } from "../breakpoints"
import Icon from "./Icon"
import Img from "./Img"
import { TextInput } from "./Inputs"
import SocialLinks from "./SocialLinks"

const StyledHeader = styled.header`
  background-color: var(--wally-white);
  margin: 0 auto;
  transition: background-color 150ms ease;
  z-index: 2;

  #nav-open {
    display: none;
  }

  @media screen and (${isMobile}) {
    width: 100%;
    height: 4rem;
    position: fixed;
  }
`

const InnerHeader = styled.div`
  height: 100%;
  padding: 10px 30px;
  margin: 0 auto;

  display: flex;
  flex-wrap: nowrap;
  justify-content: space-between;

  @media screen and (${isMobile}) {
    text-align: center;
    padding: 0;
  }
`

const LogoImageLink = styled(Link)`
  @media screen and (${notMobile}) {
    text-align: left;
    position: relative;
    max-width: 500px;
  }
`

const LogoImage = styled(Img)`
  transition: all 350ms ease;

  @media screen and (${notMobile}) {
    height: 4.8rem;

    &:hover {
      transition: all 350ms cubic-bezier(0.34, 2, 0.64, 1);
      opacity: 0.7;
    }
  }

  @media screen and (${isMobile}) {
    height: 100%;
    padding: 0.75rem 0;
    padding-left: 4rem;
  }
`

const StyledNav = styled.nav`
  display: flex;
  align-items: stretch;
  flex-wrap: nowrap;
  font-weight: 900;

  @media screen and (${isCondensed}) {
    flex-grow: 1;
  }

  @media screen and (${isMobile}) {
    display: flex;

    background: white;
    flex-direction: column;
    position: fixed;
    top: 0;
    padding: 4rem 2rem;
    left: 0;
    bottom: 0;
    transition: transform 300ms cubic-bezier(0.34, 2, 0.64, 1), visibility 300ms;
    transform: translateX(-100%);
    visibility: hidden;

    &::before {
      content: "";
      position: absolute;
      right: 90%;
      width: 100%;
      background: white;
      top: 0;
      bottom: 0;
    }

    #nav-open:checked ~ & {
      transform: translateX(0);
      visibility: visible;
    }
  }

  & > * {
    padding: 0 1rem;
    flex-grow: 1;

    @media screen and (${isCondensed}) {
      padding: 0 0.4rem;
    }

    @media screen and (${isMobile}) {
      flex-grow: 0;
    }
  }

  & > a {
    position: relative;
    padding: 1.9rem 0.65rem 1.55rem 0.65rem;
    letter-spacing: 0.3px;
    font-size: 1.6rem;
    min-width: 6.4rem;
    text-align: center;
    align-self: flex-end;

    @media screen and (${isCondensed}) and (${notMobile}) {
      font-size: 1rem;
      padding: 1.9rem 0.2rem 1.55rem 0.2rem;
      min-width: 0;
    }

    @media screen and (${isMobile}) {
      padding: 0.5rem 2rem;
      text-align: left;
    }

    &::after {
      content: " ";
      display: block;
      position: absolute;
      height: 1.4rem;
      border-right: 2px solid var(--wally-mauve);
      right: 0;
      top: 52%;
      transform: translateY(-50%);

      @media screen and (${isCondensed}) {
        display: none;
      }
    }
  }
`

const activeClassName = "nav-active"

const StyledNavLink = styled<any>(NavLink).attrs({
  activeClassName,
})`
  && {
    ${(props) => props.$styles}
  }

  &.${activeClassName} {
    color: var(--wally-red);
  }

  &:hover {
    color: var(--wally-red);
  }
`

const StyledSocialLinks = styled(SocialLinks)`
  @media screen and (${isCondensed}) {
    font-size: 1rem;

    a {
      flex-grow: 1;
      padding-left: 0.2rem;
      padding-right: 0.2rem;
    }
  }
`

const HamburgerButton = styled.label`
  position: absolute;
  left: 1rem;
  top: 1rem;
  z-index: 3;
  color: var(--wally-red);
  font-size: 1.5rem;
  line-height: 2.5rem;
  display: none;

  @media screen and (${isMobile}) {
    display: block;
  }

  #nav-open:checked ~ & {
    color: var(--wally-mauve);
  }
`

const Curtain = styled.div`
  background: rgba(255, 255, 255, 0.5);
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  position: fixed;
  display: none;
  visibility: hidden;
  transition: opacity 300ms ease, visibility 300ms;
  opacity: 0;

  @media screen and (${isMobile}) {
    display: block;
  }

  #nav-open:checked ~ & {
    visibility: visible;
    opacity: 1;
  }
`

const MobilePushDown = styled.div`
  @media screen and (${isMobile}) {
    height: 4rem;
  }
`

const SearchBarWrapper = styled.form`
  flex-grow: 2;
  padding: 1.9rem 0.65rem 1.55rem;

  @media screen and (${isMobile}) {
    padding: 0.9rem 1.65rem 0.55rem;
  }
`

const links = [
  ["Install", "/install"],
  ["Policies", "/policies"],
] as const

export default function Header() {
  const history = useHistory()
  const [searchValue, setSearchValue] = useState("")

  return (
    <>
      <StyledHeader>
        <InnerHeader>
          <input id="nav-open" type="checkbox" />
          <HamburgerButton
            htmlFor="nav-open"
            aria-haspopup
            role="button"
            tabIndex={0}
            onKeyUp={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                const checkbox = document.getElementById(
                  "nav-open"
                ) as HTMLInputElement
                checkbox.checked = !checkbox.checked
              }
            }}
          >
            <Icon icon="hamburger" />
          </HamburgerButton>

          <LogoImageLink to="/">
            <LogoImage src={logo} alt="Wally" />
          </LogoImageLink>

          <SearchBarWrapper
            onSubmit={(e) => {
              e.preventDefault()
              history.push(`/search?q=${searchValue}`)
            }}
          >
            <TextInput
              icon="search"
              placeholder="Search..."
              value={searchValue}
              onChange={(e) => {
                setSearchValue(e)
              }}
            />
          </SearchBarWrapper>

          <Curtain
            onClick={() =>
              ((document.getElementById(
                "nav-open"
              ) as HTMLInputElement).checked = false)
            }
          />
          <StyledNav>
            {links.map(([text, url]) => (
              <StyledNavLink
                activeClassName={activeClassName}
                to={url}
                key={url}
              >
                {text}
              </StyledNavLink>
            ))}

            <StyledSocialLinks />
          </StyledNav>
        </InnerHeader>
      </StyledHeader>
      <MobilePushDown />
    </>
  )
}
