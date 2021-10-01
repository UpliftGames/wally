// Custom static site generator using Parcel.
//
// This script cannot be run directly by Node.js, but can be built via Parcel
// and then run.

import fs from "fs/promises"
import { JSDOM } from "jsdom"
import Parcel from "parcel-bundler"
import path from "path"
import React from "react"
import ReactDOMServer from "react-dom/server"
import { Helmet } from "react-helmet"
import { StaticRouter as Router } from "react-router-dom"
import rimraf from "rimraf"
import sharp from "sharp"
import { ServerStyleSheet } from "styled-components"
import XmlSitemap from "xml-sitemap"
import App from "./src/App"
import jobs from "./src/jobs"
import news from "./src/news"

// Enables GitHub Pages specific configuration: CNAME and .nojekyll files.
const IS_GITHUB_PAGES = false

// Set to a domain name to have the generator automatically configure a domain.
const CNAME = "wally.run"

const BASE_URL = process.env.PARCEL_PUBLIC_URL

async function main() {
  // A list of the initial pages that should be built. The code as-written
  // will crawl links to discover any pages linked from these initial
  // routes, as well.
  const initialRoutes = [
    "/",

    // We can pass an object to override the output path of this route,
    // like if our static content host has specific requirements on 404
    // pages.
    {
      route: "/404",
      outputPath: "/404.html",
      excludeFromSitemap: true,
    },

    { route: "/letter-from-founders", excludeFromSitemap: true }, // redirect for blog post
    { route: "/jobs", excludeFromSitemap: true }, // redirect for /careers

    ...jobs.map((job) => `/careers/${job.id}`),
    ...news.map((post) => `/news/${post.path}`),
  ]

  await build({
    entry: "src/index.html",
    outDir: "dist",
    initialRoutes,

    // Each page will be rendered by this function, defined below.
    renderPage,
  })
}

// renderPage is expected to render the passed in route and render it to
// a string containing HTML.
//
// We can do that however we want. Here we use React.
//
// `template` is given to us as an HTML string that has been processed
// by Parcel. It'll be the contents of `src/index.html`, but with CSS
// and JS references injected into it.
function renderPage(template, route, addRoute) {
  const context = {}
  const sheet = new ServerStyleSheet()

  const content = ReactDOMServer.renderToString(
    sheet.collectStyles(
      <Router location={route} context={context} basename={BASE_URL}>
        <App />
      </Router>
    )
  )

  const helmet = Helmet.renderStatic()
  const headMetadata =
    helmet.meta.toString() + helmet.title.toString() + helmet.link.toString()

  const styleTags = sheet.getStyleTags()
  sheet.seal()

  const dom = new JSDOM(template)

  const main = dom.window.document.querySelector("#app")
  main.innerHTML = content

  const head = dom.window.document.querySelector("head")
  head.insertAdjacentHTML("afterbegin", headMetadata)
  head.insertAdjacentHTML("beforeend", styleTags)

  if (context.url != null) {
    const redirect = dom.window.document.createElement("meta")
    redirect.setAttribute("http-equiv", "refresh")
    redirect.setAttribute("content", `0; URL='${context.url}'`)

    head.appendChild(redirect)

    console.log("...redirects to", context.url)
  }

  // To add new pages, we can invoke `addRoute` with a site-relative
  // path.
  //
  // Here, we're just crawling the page for anything that looks like a
  // link.
  const links = dom.window.document.querySelectorAll("a")
  for (const link of links) {
    let url = link.href

    // Rough pattern to ignore off-site links
    if (/^\w+:/.test(url)) {
      continue
    }

    if (url.startsWith(BASE_URL)) {
      url = url.slice(BASE_URL.length)
    }

    addRoute(url)
  }

  return dom.serialize()
}

// Build the given Parcel entrypoint. Individual pages are rendered using the
// `renderPage` callback, which should return HTML strings.
async function build({ entry, outDir, initialRoutes, renderPage }) {
  await rimrafPromise(outDir)
  await fs.mkdir(outDir, { recursive: true })

  const publicUrl = process.env.PARCEL_PUBLIC_URL || "/"

  console.log("Generating template with Parcel...")

  const bundler = new Parcel([entry], {
    publicUrl,
    watch: false,
    minify: true,
    autoInstall: false,
    logLevel: 2,
  })

  await bundler.bundle()

  // This should be `entry`, but processed by Parcel.
  const template = await fs.readFile(path.join(outDir, path.basename(entry)))

  const visitedRoutes = new Set()
  const sitemapRoutes = new Set()
  const routesToVisit = []

  const addRoute = (route) => {
    const url = (typeof route === "string" ? route : route.route).replace(
      /^.*(\/)$/,
      ""
    )

    if (!visitedRoutes.has(url)) {
      visitedRoutes.add(url)
      routesToVisit.push(route)

      if (typeof route !== "object" || !route.excludeFromSitemap) {
        sitemapRoutes.add(url)
      }
    }
  }

  initialRoutes.forEach((route) => addRoute(route))

  while (true) {
    let route = routesToVisit.pop()

    if (route == null) {
      break
    }

    let outputPath

    if (typeof route == "string") {
      outputPath = routeToFilePath(route)
    } else {
      outputPath = route.outputPath ?? routeToFilePath(route.route)
      route = route.route
    }

    outputPath = path.join(outDir, outputPath)

    console.log(`Generating route ${route}`)
    const rendered = renderPage(template, route, addRoute)

    console.log(`Saving to ${outputPath}`)
    await fs.mkdir(path.dirname(outputPath), { recursive: true })
    await fs.writeFile(outputPath, rendered)
  }

  if (IS_GITHUB_PAGES) {
    if (CNAME != null) {
      console.log("Writing CNAME file for GitHub Pages")
      const cnamePath = path.join(outDir, "CNAME")
      await fs.writeFile(cnamePath, CNAME)
    }

    console.log("Writing .nojekyll file for GitHub Pages")
    const noJekyllPath = path.join(outDir, ".nojekyll")
    await fs.writeFile(noJekyllPath, "")
  }

  console.log("Generating sitemap...")

  const sitemap = new XmlSitemap()
  sitemap.setHost(`https://${CNAME}/`)

  Array.from(sitemapRoutes).forEach((url) =>
    sitemap.add(`https://${CNAME}` + url)
  )

  await fs.writeFile(path.join(outDir, "sitemap.xml"), sitemap.xml)

  console.log("Optimizing images...")
  await Promise.all(
    (await fs.readdir(outDir))
      .filter((fileName) => fileName.endsWith(".png"))
      .map(async (fileName) => {
        const filePath = path.join(outDir, fileName)
        const filePathNoExt = filePath.substr(0, filePath.length - 4)

        const fileBuffer = await fs.readFile(filePath)

        const image = sharp(fileBuffer)

        return Promise.all([
          image
            .avif({
              quality: 65,
              chromaSubsampling: "4:4:4",
            })
            .toFile(`${filePathNoExt}.avif`),
          image.webp({}).toFile(`${filePathNoExt}.webp`),
        ])
      })
  )
}

// Simple Promise wrapper around rimraf.
function rimrafPromise(dir) {
  return new Promise((resolve, reject) => {
    rimraf(dir, (err) => {
      if (err == null) {
        resolve()
      } else {
        reject(err)
      }
    })
  })
}

// Transforms a site-relative route for a page into a file path.
function routeToFilePath(route) {
  let current = ""

  for (const piece of route.split("/")) {
    if (piece.length === 0) {
      continue
    }

    current = path.join(current, piece)
  }

  return path.join(current, "index.html")
}

main()
