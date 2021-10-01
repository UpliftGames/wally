import fastify from "fastify"
import fastifyStatic from "fastify-static"
import { createReadStream, readFileSync } from "fs"
import path from "path"
import React from "react"
import ReactDOMServer from "react-dom/server"
import { Helmet } from "react-helmet"
import { StaticRouter as Router } from "react-router-dom"
import { ServerStyleSheet } from "styled-components"
import App from "./App"

const staticFolder = path.join(__dirname, "../static")
const index = readFileSync(path.join(staticFolder, "index.html"), "utf8")

const renderRoute = (request, reply) => {
  const context = {}
  const sheet = new ServerStyleSheet()

  const body = ReactDOMServer.renderToString(
    sheet.collectStyles(
      <Router location={request.url} context={context}>
        <App />
      </Router>
    )
  )

  const helmet = Helmet.renderStatic()
  const headMetadata =
    helmet.meta.toString() + helmet.title.toString() + helmet.link.toString()

  const styleTags = sheet.getStyleTags()
  sheet.seal()

  if (context.statusCode != null) {
    reply.code(context.statusCode)
  }

  return index
    .replace(`<head>`, `<head>${headMetadata}`)
    .replace(`</head>`, `${styleTags}</head>`)
    .replace(`<div id="app"></div>`, `<div id="app">${body}</div>`)
}

const app = fastify({ logger: true })

app.register(fastifyStatic, {
  root: staticFolder,
  prefix: "/static",
})

app.get("/favicon.ico", async (request, reply) => {
  const stream = createReadStream(path.join(staticFolder, "favicon.ico"))

  reply.type("image/x-icon")
  reply.send(stream)
})

app.get("/robots.txt", async (request, reply) => {
  const stream = createReadStream(path.join(staticFolder, "robots.txt"))

  reply.type("text/plain")
  reply.send(stream)
})

app.get("/*", async (request, reply) => {
  reply.type("text/html")

  return renderRoute(request, reply)
})

const start = async () => {
  try {
    await app.listen(8000, "0.0.0.0")
  } catch (err) {
    app.log.error(err)
    process.exit(1)
  }
}

start()
