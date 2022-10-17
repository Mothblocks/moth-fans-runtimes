import React, { useRef } from "react"
import { useNavigate, useParams } from "react-router-dom"
import { Round } from "../data"
import { runtimeToKey } from "../runtimeToKey"

const PATTERN_CODE_FILE = /tgstation\/tree\/.+?\/(.+)$/

const extractCodeFile = (filename: string) => {
  const match = filename.match(PATTERN_CODE_FILE)
  return match ? match[1] : filename
}

export const RuntimeViewer = ({ rounds }: { rounds: Round[] }) => {
  const { runtimeKey } = useParams()
  const navigate = useNavigate()
  const ref = useRef<HTMLDivElement>(null)

  const foundRounds = new Map<Round, number>()

  const masterFilenames = new Set<string>()
  const filenames = new Set<string>()

  const addFilename = (filename: string, revision: string, line: number) => {
    filenames.add(
      `https://github.com/tgstation/tgstation/tree/${revision}/${filename}#L${line}`
    )

    masterFilenames.add(
      `https://github.com/tgstation/tgstation/tree/master/${filename}#L${line}`
    )
  }

  let exception = "Couldn't find runtime"

  // const runtime
  for (const round of rounds) {
    if (!round.runtimes) {
      continue
    }

    for (const runtime of round.runtimes) {
      if (runtimeToKey(runtime) !== runtimeKey) {
        continue
      }

      exception = runtime.exception

      foundRounds.set(round, runtime.count)

      const bestGuessFilenames = runtime.best_guess_filenames

      if (bestGuessFilenames) {
        if ("Definitely" in bestGuessFilenames) {
          addFilename(
            bestGuessFilenames.Definitely,
            round.revision,
            runtime.line
          )
        } else if ("Possible" in bestGuessFilenames) {
          for (const filename of bestGuessFilenames.Possible) {
            addFilename(filename, round.revision, runtime.line)
          }
        }
      }
    }
  }

  const sortedRounds = Array.from(foundRounds).sort(
    ([, countA], [, countB]) => {
      return countB - countA
    }
  )

  return (
    <div
      ref={ref}
      onClick={(event) => {
        if (
          event.target === ref.current ||
          event.target instanceof HTMLLIElement ||
          event.target instanceof HTMLUListElement
        ) {
          navigate("/")
        }
      }}
      style={{
        background: "rgba(255, 255, 255, 0.9)",

        boxSizing: "border-box",
        padding: "10px",

        position: "absolute",
        top: 0,

        height: "104%",
        width: "100%",
      }}
    >
      <div>
        <h1>{exception}</h1>
      </div>

      <div
        style={{
          display: "flex",
          height: "50vh",
        }}
      >
        <ul
          style={{
            flexGrow: 1,
            overflowY: "scroll",
          }}
        >
          {sortedRounds.map(([round, count]) => (
            <li key={round.round_id}>
              <a
                href={`https://scrubby.melonmesa.com/round/${round.round_id}/source`}
                target="_blank"
                rel="noreferrer"
              >
                {round.round_id} - {count.toLocaleString()}
              </a>
            </li>
          ))}
        </ul>

        <ul
          style={{
            flexGrow: 1,
            overflowY: "scroll",
          }}
        >
          {Array.from(masterFilenames).map((filename) => (
            <li key={filename}>
              <a href={filename}>master - {extractCodeFile(filename)}</a>
            </li>
          ))}

          <hr />

          {Array.from(filenames).map((filename) => (
            <li key={filename}>
              <a href={filename}>
                {filename
                  .match(/tree\/([a-z0-9]+)/)!
                  .at(1)
                  ?.substring(0, 6)}{" "}
                - {extractCodeFile(filename)}
              </a>
            </li>
          ))}
        </ul>
      </div>
    </div>
  )
}
