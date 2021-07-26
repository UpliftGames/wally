import React from "react"
import ReactDOM from "react-dom"
import { BrowserRouter as Router } from "react-router-dom"
import App from "./components/App"
import "./index.css"
import "./theme.css"

const main = document.querySelector("#app")

const app = (
  <Router>
    <App />
  </Router>
)

if (main.hasChildNodes()) {
  ReactDOM.hydrate(app, main)
} else {
  ReactDOM.render(app, main)
}
