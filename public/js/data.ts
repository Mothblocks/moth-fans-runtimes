export type Round = {
  round_id: number
  timestamp: string
  revision: string
  server: string

  runtimes?: RuntimeBatch[]
  test_merges: TestMerge[]
}

export type RuntimeBatch = {
  count: number
  exception: string
  proc_path: string
  source_file: string
  line: number

  best_guess_filenames?: BestGuessFilenames
}

export type BestGuessFilenames =
  | {
      Definitely: string
    }
  | {
      Possible: string[]
    }

export type TestMerge = {
  details: TestMergeDetails
  files_changed: string[]
}

export type TestMergeDetails = {
  number: number
  title: string
  author: string
  commit: string
}
