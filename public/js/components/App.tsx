import React, { useEffect, useState } from "react"
import { BrowserRouter as Router, Routes, Route } from "react-router-dom"
import { Chart, BarElement, CategoryScale, LinearScale } from "chart.js"
import { Round } from "../data"
import { Loading } from "./Loading"
import { Main } from "./Main"

Chart.register(BarElement, CategoryScale, LinearScale)

export const App = () => {
  const [rounds, setRounds] = useState<readonly Round[] | undefined>(undefined)
  const [loadError, setLoadError] = useState<string | undefined>(undefined)

  useEffect(() => {
    fetch("/data.json")
      .then(async (response) => {
        if (response.ok) {
          const roundsData: Round[] = await response.json()
          roundsData.reverse()
          return Object.freeze(roundsData)
        } else {
          throw new Error(response.statusText)
        }
      })
      .then(setRounds)
      .catch((error) => setLoadError(error.message))
  }, [])

  return (
    <React.StrictMode>
      <Router>
        <Routes>
          <Route
            path="*"
            element={
              rounds ? <Main rounds={rounds} /> : <Loading error={loadError} />
            }
          />
        </Routes>
      </Router>
    </React.StrictMode>
  )
}
