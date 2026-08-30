import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname } from 'node:path'

export function writeOrCheckGeneratedFile({ check, content, displayPath, outputFile }) {
  if (check) {
    let actual
    try {
      actual = readFileSync(outputFile, 'utf8')
    } catch (error) {
      if (error?.code !== 'ENOENT') throw error
      actual = null
    }

    if (actual !== content) {
      throw new Error(
        `Generated documentation drift detected in ${displayPath}.\n` +
          'Run `pnpm docs:gen` and commit the updated generated documentation.'
      )
    }
    return
  }

  mkdirSync(dirname(outputFile), { recursive: true })
  writeFileSync(outputFile, content, 'utf8')
}
