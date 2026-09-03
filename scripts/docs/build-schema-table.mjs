#!/usr/bin/env node
// scripts/docs/build-schema-table.mjs
//
// 扫描 src-tauri/src/db/schema/*.rs 中按源码顺序出现的 schema DDL，
// 归并 CREATE TABLE / ALTER TABLE ADD COLUMN / CREATE [UNIQUE] INDEX / DROP INDEX
// 得到每张表的最终列与索引，生成 docs/architecture/_generated/data-model.md。
// 在 docs:gen 中调用，配合改 schema 后一并刷新文档。

import { readdirSync, readFileSync } from 'node:fs'
import { join, relative, resolve, sep } from 'node:path'
import { pathToFileURL } from 'node:url'
import { writeOrCheckGeneratedFile } from './generated-doc-file.mjs'
import { resolveRepoRoot } from '../lib/repo-root.mjs'

const repoRoot = resolveRepoRoot(import.meta.url)
const schemaDir = join(repoRoot, 'src-tauri', 'src', 'db', 'schema')
const outDir = join(repoRoot, 'docs', 'architecture', '_generated')
const outFile = join(outDir, 'data-model.md')

const IDENT = '[A-Za-z_][A-Za-z0-9_]*'

function shortPath(absolute, rootDir = repoRoot) {
  return relative(rootDir, absolute).split(sep).join('/')
}

function lineNumberAt(text, index) {
  let line = 1
  for (let i = 0; i < index; i++) {
    if (text[i] === '\n') line++
  }
  return line
}

function extractRustStrings(text) {
  const strings = []
  let i = 0
  while (i < text.length) {
    const ch = text[i]
    const next = text[i + 1]
    if (ch === '/' && next === '/') {
      const newline = text.indexOf('\n', i)
      if (newline < 0) break
      i = newline + 1
      continue
    }
    if (ch === '/' && next === '*') {
      const end = text.indexOf('*/', i + 2)
      if (end < 0) break
      i = end + 2
      continue
    }
    if (ch === '"') {
      const start = i
      i += 1
      let value = ''
      while (i < text.length) {
        if (text[i] === '\\' && i + 1 < text.length) {
          const escaped = text[i + 1]
          switch (escaped) {
            case 'n':
              value += '\n'
              break
            case 'r':
              value += '\r'
              break
            case 't':
              value += '\t'
              break
            case '"':
            case '\\':
              value += escaped
              break
            default:
              value += escaped
              break
          }
          i += 2
          continue
        }
        if (text[i] === '"') {
          strings.push({ value, line: lineNumberAt(text, start) })
          i += 1
          break
        }
        value += text[i]
        i += 1
      }
      continue
    }
    i += 1
  }
  return strings
}

function parseColumns(body) {
  const parts = []
  let depth = 0
  let buf = ''
  for (const ch of body) {
    if (ch === '(') depth++
    if (ch === ')') depth--
    if (ch === ',' && depth === 0) {
      parts.push(buf)
      buf = ''
    } else {
      buf += ch
    }
  }
  if (buf.length) parts.push(buf)

  const cols = []
  let pkComposite = null
  for (const raw of parts) {
    const trimmed = raw.trim().replace(/\s+/g, ' ')
    if (!trimmed) continue
    if (/^PRIMARY KEY\b/i.test(trimmed)) {
      pkComposite = trimmed
        .replace(/^PRIMARY KEY\s*\(/i, '')
        .replace(/\)$/, '')
        .split(',')
        .map((s) => s.trim())
      continue
    }
    if (/^(?:CHECK|FOREIGN KEY|UNIQUE)\b/i.test(trimmed)) continue
    const tokens = trimmed.split(/\s+/)
    const name = tokens.shift()
    const rest = tokens.join(' ')
    cols.push({
      name,
      type: rest
        .replace(/PRIMARY KEY/i, '')
        .replace(/NOT NULL/i, '')
        .replace(/DEFAULT [^,]+/i, '')
        .trim(),
      notNull: /NOT NULL/i.test(rest),
      defaultExpr: extractDefault(rest),
      isPk: /PRIMARY KEY/i.test(rest),
    })
  }
  if (pkComposite) {
    for (const col of cols) {
      if (pkComposite.includes(col.name)) col.isPk = true
    }
  }
  return cols
}

function extractDefault(text) {
  const m = text.match(/DEFAULT\s+([^,]+?)(?:\s+NOT NULL)?$/i)
  if (!m) return null
  return m[1].trim()
}

function normalizeSql(value) {
  return value.trim().replace(/;\s*$/, '')
}

function ddlKind(sql) {
  const upper = sql.replace(/\s+/g, ' ')
  if (/^CREATE UNIQUE INDEX\b/i.test(upper)) return 'CREATE UNIQUE INDEX'
  if (/^CREATE INDEX\b/i.test(upper)) return 'CREATE INDEX'
  if (/^CREATE TABLE\b/i.test(upper)) return 'CREATE TABLE'
  if (/^ALTER TABLE\b/i.test(upper)) return 'ALTER TABLE'
  if (/^DROP INDEX\b/i.test(upper)) return 'DROP INDEX'
  return null
}

function unsupportedDdl(kind, loc) {
  return new Error(`[build-schema-table] unsupported ${kind} in ${loc}`)
}

function conflictDdl(kind, loc, detail) {
  return new Error(`[build-schema-table] conflicting ${kind} in ${loc}: ${detail}`)
}

function parseCreateTable(sql) {
  const matched = sql.match(
    new RegExp(`^CREATE TABLE IF NOT EXISTS\\s+(${IDENT})\\s*\\(([\\s\\S]*)\\)\\s*$`, 'i')
  )
  if (!matched) return null
  return { name: matched[1], cols: parseColumns(matched[2]) }
}

function parseAlterAddColumn(sql) {
  const matched = sql.match(
    new RegExp(`^ALTER TABLE\\s+(${IDENT})\\s+ADD COLUMN\\s+([\\s\\S]+)$`, 'i')
  )
  if (!matched) return null
  const cols = parseColumns(matched[2])
  if (cols.length !== 1 || !cols[0].name) return null
  return { table: matched[1], column: cols[0] }
}

function parseCreateIndex(sql) {
  const matched = sql.match(
    new RegExp(
      `^CREATE\\s+(UNIQUE\\s+)?INDEX IF NOT EXISTS\\s+(${IDENT})\\s+ON\\s+(${IDENT})\\s*\\(([\\s\\S]*)\\)\\s*$`,
      'i'
    )
  )
  if (!matched) return null
  return {
    unique: Boolean(matched[1]),
    name: matched[2],
    table: matched[3],
    columns: matched[4]
      .split(',')
      .map((part) => part.trim())
      .filter(Boolean),
  }
}

function parseDropIndex(sql) {
  const matched = sql.match(new RegExp(`^DROP INDEX IF EXISTS\\s+(${IDENT})\\s*$`, 'i'))
  if (!matched) return null
  return { name: matched[1] }
}

function addIndex(tablesByName, indexesByName, index, loc, kind) {
  const table = tablesByName.get(index.table)
  if (!table) {
    throw new Error(`[build-schema-table] ${kind} in ${loc} references unknown table ${index.table}`)
  }
  if (indexesByName.has(index.name)) {
    throw conflictDdl(kind, loc, `index '${index.name}' already defined`)
  }
  const record = { name: index.name, columns: index.columns, unique: index.unique }
  indexesByName.set(index.name, { table: index.table, record })
  table.indexOrder.push(index.name)
}

function dropIndex(tablesByName, indexesByName, name) {
  const existing = indexesByName.get(name)
  if (!existing) return
  const table = tablesByName.get(existing.table)
  if (table) {
    table.indexOrder = table.indexOrder.filter((indexName) => indexName !== name)
  }
  indexesByName.delete(name)
}

export function parseFile(filePath, rootDir = repoRoot) {
  const text = readFileSync(filePath, 'utf8')
  const source = shortPath(filePath, rootDir)
  const tablesByName = new Map()
  const tableOrder = []
  const indexesByName = new Map()

  for (const stmt of extractRustStrings(text)) {
    const sql = normalizeSql(stmt.value)
    const kind = ddlKind(sql)
    if (!kind) continue
    const loc = `${source}:${stmt.line}`

    switch (kind) {
      case 'CREATE TABLE': {
        const parsed = parseCreateTable(sql)
        if (!parsed) throw unsupportedDdl(kind, loc)
        if (tablesByName.has(parsed.name)) {
          throw conflictDdl(kind, loc, `table '${parsed.name}' already defined`)
        }
        tablesByName.set(parsed.name, { cols: parsed.cols, indexOrder: [] })
        tableOrder.push(parsed.name)
        break
      }
      case 'ALTER TABLE': {
        const parsed = parseAlterAddColumn(sql)
        if (!parsed) throw unsupportedDdl(kind, loc)
        const table = tablesByName.get(parsed.table)
        if (!table) {
          throw new Error(
            `[build-schema-table] ${kind} in ${loc} references unknown table ${parsed.table}`
          )
        }
        // CREATE TABLE is canonical for columns it already declared; ALTER ADD
        // only fills gaps so old-DB ensure_column strings cannot weaken new-DB DDL.
        if (!table.cols.some((col) => col.name === parsed.column.name)) {
          table.cols.push(parsed.column)
        }
        break
      }
      case 'CREATE INDEX':
      case 'CREATE UNIQUE INDEX': {
        const parsed = parseCreateIndex(sql)
        if (!parsed) throw unsupportedDdl(kind, loc)
        addIndex(tablesByName, indexesByName, parsed, loc, kind)
        break
      }
      case 'DROP INDEX': {
        const parsed = parseDropIndex(sql)
        if (!parsed) throw unsupportedDdl(kind, loc)
        dropIndex(tablesByName, indexesByName, parsed.name)
        break
      }
      default: {
        const _exhaustive = kind
        throw unsupportedDdl(_exhaustive, loc)
      }
    }
  }

  const tables = tableOrder.map((name) => {
    const table = tablesByName.get(name)
    return {
      name,
      cols: table.cols,
      indexes: table.indexOrder.map((indexName) => indexesByName.get(indexName).record),
    }
  })
  return { source, tables }
}

function escapePipe(text) {
  return String(text || '').replace(/\|/g, '\\|')
}

export function render(modules) {
  const totalTables = modules.reduce((acc, m) => acc + m.tables.length, 0)
  const out = []
  out.push('<!-- AUTOGENERATED — do not edit by hand. -->')
  out.push('<!-- regenerate via: pnpm docs:gen -->')
  out.push('')
  out.push(`> Schema modules: ${modules.length}　·　Tables: ${totalTables}`)
  out.push('')
  for (const mod of modules) {
    out.push(`## \`${mod.source}\``)
    out.push('')
    for (const table of mod.tables) {
      out.push(`### \`${table.name}\``)
      out.push('')
      out.push('| Column | Type | Nullable | Default | PK |')
      out.push('| --- | --- | --- | --- | --- |')
      for (const col of table.cols) {
        out.push(
          `| \`${col.name}\` | \`${escapePipe(col.type)}\` | ${col.notNull ? 'no' : 'yes'} | ${
            col.defaultExpr ? `\`${escapePipe(col.defaultExpr)}\`` : '—'
          } | ${col.isPk ? '✓' : '—'} |`
        )
      }
      out.push('')
      if (table.indexes.length) {
        out.push('Indexes:')
        for (const ix of table.indexes) {
          out.push(
            `- \`${ix.name}\` on \`(${ix.columns.join(', ')})\` ${ix.unique ? 'unique' : 'non-unique'}`
          )
        }
        out.push('')
      }
    }
  }
  return out.join('\n')
}

export function generateSchemaDocs({
  check = false,
  log = console.log,
  outputFile = outFile,
  rootDir = repoRoot,
  sourceDir = schemaDir,
} = {}) {
  const files = readdirSync(sourceDir)
    .filter((f) => f.endsWith('.rs') && f !== 'mod.rs')
    .sort()
    .map((f) => join(sourceDir, f))
  const modules = files.map((file) => parseFile(file, rootDir)).filter((m) => m.tables.length > 0)
  if (!modules.length) {
    throw new Error('[build-schema-table] no CREATE TABLE statements found - refusing to overwrite')
  }
  const md = render(modules)
  const total = modules.reduce((a, m) => a + m.tables.length, 0)
  const displayPath = shortPath(outputFile, rootDir)
  writeOrCheckGeneratedFile({ check, content: md, displayPath, outputFile })
  log(
    check
      ? `[build-schema-table] up to date: ${displayPath}`
      : `[build-schema-table] wrote ${total} tables -> ${displayPath}`
  )
  return md
}

function runCli(args) {
  if (args.some((arg) => arg !== '--check')) {
    throw new Error('[build-schema-table] usage: node scripts/docs/build-schema-table.mjs [--check]')
  }
  generateSchemaDocs({ check: args.includes('--check') })
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try {
    runCli(process.argv.slice(2))
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error))
    process.exitCode = 1
  }
}
