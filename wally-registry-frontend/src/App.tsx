import React from "react"
import { Helmet } from "react-helmet"
import { Route, Switch } from "react-router-dom"
import "regenerator-runtime/runtime"
import { StyleSheetManager } from "styled-components"
import CanonicalTag from "./components/CanonicalTag"
import ContentSection from "./components/ContentSection"
import Footer from "./components/Footer"
import Header from "./components/Header"
import ScrollToTop from "./components/ScrollToTop"
import MDXComponents from "./MDXComponents"
import Home from "./pages/Home"
import Install from "./pages/Install.mdx"
import NotFound from "./pages/NotFound"
import Package from "./pages/Package"
import Policies from "./pages/Policies.mdx"

export default function App() {
  return (
    <MDXComponents>
      <StyleSheetManager disableVendorPrefixes>
        <>
          <Helmet titleTemplate="%s | Wally" defaultTitle="Wally">
            <meta charSet="utf-8" />
            <meta
              name="viewport"
              content="width=device-width, initial-scale=1, shrink-to-fit=no"
            />
            <meta
              name="description"
              content="Wally is a modern package manager for Roblox projects inspired by Cargo"
            />
            {/* <meta name="og:image" content={embedImage} /> */}
            <meta name="theme-color" content="#FF6A8B" />
          </Helmet>
          <ScrollToTop />
          <CanonicalTag />

          <Header />

          <Switch>
            <Route exact path="/">
              <Home />
            </Route>

            {/* <Route path="/search">
              <Search />
            </Route> */}

            <Route path="/package/:packageScope/:packageName">
              <Package />
            </Route>

            <Route path="/install">
              <Helmet>
                <title>Install</title>
              </Helmet>
              <ContentSection>
                <Install />
              </ContentSection>
            </Route>

            <Route path="/policies">
              <Helmet>
                <title>Policies</title>
              </Helmet>
              <ContentSection>
                <Policies />
              </ContentSection>
            </Route>

            <Route
              path="*"
              render={({ staticContext }) => {
                if (staticContext) staticContext.statusCode = 404
                return <NotFound />
              }}
            />
          </Switch>

          <Footer />
        </>
      </StyleSheetManager>
    </MDXComponents>
  )
}
