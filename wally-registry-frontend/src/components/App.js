import React from "react"
import style from "./App.css"

export default function App() {
  return (
    <>
      <header className={ style.Header }>
        <h1>
          Wally Package Registry
        </h1>
      </header>
      <main className={ style.Main }>
        <div className={ style.CenterContent }>
          <button className={ style.PlaceholderButton } onClick={ () => console.log("Hello, world!") }>Print Hello World</button>
        </div>
      </main>
    </>
  )
}
