import React, { useState } from "react"
import { Bar } from "react-chartjs-2"
import { Round, RuntimeBatch } from "../data"
import { RuntimeTable } from "./RuntimeTable"
import { Route, Routes } from "react-router-dom"
import { RuntimeViewer } from "./RuntimeViewer"

const SERVER_COLORS: Record<string, string> = {
  basil: "hsl(20, 100%, 15%)",
  sybil: "hsl(245, 100%, 70%)",
  manuel: "hsl(20, 100%, 50%)",
  terry: "hsl(0, 100%, 30%)",
  campbell: "hsl(275, 100%, 70%)",
  "event-hall-us": "hsl(100, 100%, 70%)",
  unknown: "hsl(0, 0%, 30%)",
}

const ServerFilter = ({
  serverFilter,
  setServerFilter,
}: {
  serverFilter: string
  setServerFilter: (serverFilter: string) => void
}) => {
  return (
    <select
      value={serverFilter}
      onChange={(event) => setServerFilter(event.target.value)}
    >
      <option value="all">all servers</option>

      {Object.keys(SERVER_COLORS)
        .filter((server) => server !== "unknown")
        .map((server) => (
          <option key={server} value={server}>
            {server}
          </option>
        ))}
    </select>
  )
}

const Timeframe = ({
  timeframe,
  setTimeframe,
}: {
  timeframe: number
  setTimeframe: (timeframe: number) => void
}) => {
  return (
    <select
      value={timeframe}
      onChange={(event) => setTimeframe(parseInt(event.target.value))}
    >
      <option value={7}>last week</option>
      <option value={3}>last three days</option>
      <option value={1}>last day</option>
    </select>
  )
}

const RuntimeChart = ({ rounds }: { rounds: Round[] }) => {
  const labels = []
  const datasets: [string, number][] = []

  for (const round of rounds) {
    const { server, runtimes } = round

    if (!runtimes) {
      continue
    }

    labels.push(round.round_id)

    datasets.push([
      server,
      runtimes.reduce((total, batch) => total + batch.count, 0),
    ])
  }

  return (
    <Bar
      datasetIdKey="runtimes"
      data={{
        labels,

        datasets: [
          {
            backgroundColor: datasets.map(
              ([server]) => SERVER_COLORS[server] || SERVER_COLORS.unknown
            ),
            data: datasets.map(([, count]) => count),
            barPercentage: 1,
            categoryPercentage: 1,
          },
        ],
      }}
      options={{
        scales: {
          y: {
            beginAtZero: true,
            display: false,
          },
        },

        responsive: true,
        maintainAspectRatio: false,
      }}
    />
  )
}

export const Main = ({ rounds }: { rounds: readonly Round[] }) => {
  const [serverFilter, setServerFilter] = useState("all")
  const [timeframe, setTimeframe] = useState(7)
  const [collateSimilar, setCollateSimilar] = useState(true)

  const [search, setSearch] = useState("")
  const lowercaseSearch = search.toLowerCase()

  const dateOfLastRound = new Date(rounds[rounds.length - 1].timestamp)

  const collatedRuntimes: Record<string, RuntimeBatch> = {}

  console.time("calculate filteredRounds")
  const filteredRounds = rounds
    .filter((round) => {
      if (serverFilter !== "all" && round.server !== serverFilter) {
        return false
      }

      if (
        Math.abs(
          dateOfLastRound.getTime() - new Date(round.timestamp).getTime()
        ) /
          (1000 * 60 * 60 * 24) >
        timeframe
      ) {
        return false
      }

      return true
    })
    .map((round) => {
      let currentRound = round

      if (search !== "") {
        const runtimesAfterFiltering: RuntimeBatch[] = []

        for (const runtime of round.runtimes || []) {
          if (
            runtime.exception.toLowerCase().includes(lowercaseSearch) ||
            runtime.source_file.toLowerCase().includes(lowercaseSearch) ||
            runtime.proc_path.toString().includes(lowercaseSearch)
          ) {
            runtimesAfterFiltering.push(runtime)
          }
        }

        currentRound = {
          ...currentRound,
          runtimes: runtimesAfterFiltering,
        }
      }

      if (collateSimilar) {
        const runtimesAfterCollation: RuntimeBatch[] = []

        if (currentRound.runtimes) {
          const multipleCollationsThisRound: Record<string, RuntimeBatch> = {}

          for (const runtime of currentRound.runtimes) {
            const key = `${runtime.proc_path}__${runtime.source_file}__${runtime.line}`
            const existingRuntime = collatedRuntimes[key]

            if (existingRuntime) {
              let multipleThisRound = multipleCollationsThisRound[key]

              if (multipleThisRound) {
                multipleThisRound.count += runtime.count
              } else {
                multipleThisRound = {
                  ...existingRuntime,
                  count: runtime.count,
                }

                multipleCollationsThisRound[key] = multipleThisRound
                runtimesAfterCollation.push(multipleThisRound)
              }
            } else {
              const runtimeClone = { ...runtime }
              runtimesAfterCollation.push(runtimeClone)
              multipleCollationsThisRound[key] = runtimeClone
              collatedRuntimes[key] = runtimeClone
            }
          }
        }

        if (currentRound === round) {
          currentRound = { ...round, runtimes: runtimesAfterCollation }
        } else {
          currentRound.runtimes = runtimesAfterCollation
        }
      }

      return currentRound
    })
  console.timeEnd("calculate filteredRounds")

  return (
    <>
      <div
        style={{
          display: "flex",
          gap: "10px",
          position: "absolute",
          top: 5,
          left: 5,
        }}
      >
        <ServerFilter
          serverFilter={serverFilter}
          setServerFilter={setServerFilter}
        />

        <Timeframe timeframe={timeframe} setTimeframe={setTimeframe} />

        <div>
          <input
            type="checkbox"
            id="collate-similar"
            checked={collateSimilar}
            onChange={() => setCollateSimilar(!collateSimilar)}
          />

          <label htmlFor="collate-similar">collate similar</label>
        </div>
      </div>

      <div>
        <div style={{ height: "18vh", width: "100%" }}>
          <RuntimeChart rounds={filteredRounds} />
        </div>

        <div
          style={{
            height: "77vh",
            width: "100%",
            position: "relative",
          }}
        >
          <input
            placeholder="search"
            onChange={(event) => setSearch(event.target.value)}
            value={search}
            style={{
              width: "100%",
            }}
          />

          <RuntimeTable rounds={filteredRounds} />

          <Routes>
            <Route
              path={`/runtime/:runtimeKey`}
              element={<RuntimeViewer rounds={filteredRounds} />}
            />
          </Routes>
        </div>
      </div>
    </>
  )
}
