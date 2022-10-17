import React from "react"
import { Link } from "react-router-dom"
import { FixedSizeList } from "react-window"
import AutoSizer from "react-virtualized-auto-sizer"
import { Round, RuntimeBatch } from "../data"
import { runtimeToKey } from "../runtimeToKey"

const Runtime = ({
  runtime,
  index,
  totalCount,
  roundCount,
  percent,
}: {
  runtime: RuntimeBatch
  index: number
  totalCount: number
  roundCount: number
  percent: number
}) => {
  return (
    <div
      style={{
        alignItems: "center",
        backgroundColor: index % 2 === 0 ? "#fff" : "#e5e5e5",
        borderBottom: "1px solid #ccc",
        display: "flex",
        padding: "5px",
      }}
    >
      <div
        style={{
          minWidth: "10%",
          maxWidth: "10%",
        }}
      >
        {totalCount.toLocaleString()}
        <br />

        <span
          style={{
            fontSize: "0.8em",
          }}
        >
          ({roundCount.toLocaleString()} rounds, {(percent * 100).toFixed(1)}%)
        </span>
      </div>

      <div
        style={{
          flexGrow: 1,
        }}
      >
        <Link to={`/runtime/${encodeURIComponent(runtimeToKey(runtime))}`}>
          {runtime.exception}
        </Link>
      </div>

      <div
        style={{
          minWidth: "15%",
          maxWidth: "15%",
        }}
      >
        {runtime.source_file}:{runtime.line}
      </div>

      <div
        style={{
          minWidth: "15%",
          maxWidth: "15%",
        }}
      >
        {runtime.proc_path}
      </div>
    </div>
  )
}

export const RuntimeTable = ({ rounds }: { rounds: Round[] }) => {
  const runtimes: RuntimeBatch[] = []

  const roundCounts: Record<string, Set<number>> = {}
  const runtimeCounts: Record<string, number> = {}

  console.time("collect rounds for RuntimeTable")

  for (const round of rounds) {
    if (!round.runtimes) {
      continue
    }

    for (const runtimeBatch of round.runtimes) {
      const key = runtimeToKey(runtimeBatch)

      if (roundCounts[key]) {
        roundCounts[key].add(round.round_id)
      } else {
        roundCounts[key] = new Set([round.round_id])
        runtimes.push(runtimeBatch)
      }

      if (runtimeCounts[key]) {
        runtimeCounts[key] += runtimeBatch.count
      } else {
        runtimeCounts[key] = runtimeBatch.count
      }
    }
  }

  runtimes.sort((a, b) => {
    return runtimeCounts[runtimeToKey(b)] - runtimeCounts[runtimeToKey(a)]
  })

  console.timeEnd("collect rounds for RuntimeTable")

  return (
    <div
      style={{
        display: "table",
        height: "100%",
        width: "100%",
      }}
    >
      <AutoSizer>
        {({ height, width }) => (
          <FixedSizeList
            height={height}
            itemCount={runtimes.length}
            itemSize={45}
            width={width}
          >
            {({ index, style }) => {
              const roundCount =
                roundCounts[runtimeToKey(runtimes[index])]?.size || 0

              return (
                <div style={style}>
                  <Runtime
                    index={index}
                    runtime={runtimes[index]}
                    totalCount={
                      runtimeCounts[runtimeToKey(runtimes[index])] || 0
                    }
                    roundCount={roundCount}
                    percent={roundCount / rounds.length}
                  />
                </div>
              )
            }}
          </FixedSizeList>
        )}
      </AutoSizer>
    </div>
  )
}
