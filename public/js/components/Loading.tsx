import React from "react"

export const Loading = ({ error }: { error?: string }) => {
  return (
    <div
      style={{
        display: "flex",
        fontSize: error ? "2rem" : "8rem",
        height: "100vh",
        width: "100vw",

        alignItems: "center",
        flexDirection: "column",
        justifyContent: "center",
      }}
    >
      {error ? (
        <>
          <div>error loading data</div>

          <div>
            <code>{error}</code>
          </div>
        </>
      ) : (
        "🦋 loading... 🧊"
      )}
    </div>
  )
}
