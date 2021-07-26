import React from "react"
import ReactDOMServer from "react-dom/server"
import { StaticRouter as Router } from "react-router-dom"
import fastify from "fastify"
import fastifyStatic from "fastify-static"
import path from "path"
import { readFileSync } from "fs"

import App from "./components/App"

const staticFolder = path.join(__dirname, "../static")
const index = readFileSync(path.join(staticFolder, "index.html"), "utf8")

const renderRoute = route => {
  const body = ReactDOMServer.renderToString(
      <Router location={route}>
        <App />
      </Router>
  )

  return index.replace(`<div id="app"></div>`, `<div id="app">${ body }</div>`)
}

const app = fastify({ logger: true })

app.register(fastifyStatic, {
  root: staticFolder,
  prefix: "/static",
})

app.get("/", async (request, reply) => {
  reply.type("text/html")
  return renderRoute("/")
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
