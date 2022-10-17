import { RuntimeBatch } from "./data"

// Not too specific in case of file changes
export const runtimeToKey = (runtime: RuntimeBatch): string => {
  return `${runtime.exception}_______${runtime.proc_path}`
}
