import React from "react"
import ReactDOM from "react-dom"
import { BrowserRouter as Router } from "react-router-dom"
import App from "./App"
import "./fonts/icons.css"
import "./index.css"
import "./theme.css"

const main = document.querySelector("#app")

const app = (
  <Router basename={process.env.PARCEL_PUBLIC_URL}>
    <App />
  </Router>
)

if (main.hasChildNodes()) {
  ReactDOM.hydrate(app, main)
} else {
  ReactDOM.render(app, main)
}
