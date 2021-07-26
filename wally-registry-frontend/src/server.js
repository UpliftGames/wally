import React from "react"
import ReactDOMServer from "react-dom/server"
import fastify from "fastify"
import fastifyStatic from "fastify-static"
import path from "path"

import App from "./components/App"
import index from "./index.html"

const app = fastify({ logger: true })

app.register(fastifyStatic, {
  root: path.join(process.cwd(), "static"),
  prefix: "/static",
})

app.get("/", async (request, reply) => {
  const appOutput = ReactDOMServer.renderToString(<App />)
  const body = index.replace(`<div id="root"></div>`, `<div id="root">${ appOutput }</div>`)

  reply.type("text/html")
  return body
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
