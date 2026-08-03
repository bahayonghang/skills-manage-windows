# Production ingestion inventory

Final pass: 2026-08-03. Test-only reads are excluded unless they exercise a
production helper.

## Bounded HTTP and stream reads

| Call site | Final boundary |
| --- | --- |
| `services/github_import/tree_manifest.rs` | shared incremental reader, 16 MiB |
| `services/github_import/raw_http.rs` | shared incremental reader; metadata 1 MiB, repository files 32 MiB |
| `services/github_import/archive.rs` | shared incremental reader, 128 MiB |
| GitHub denial diagnostics | shared incremental reader, 64 KiB; body never enters public error |
| AI one-shot, connection test, tagging | success 1 MiB, diagnostics 64 KiB, 30 s header/body deadlines |
| `services/ai_provider/stream.rs` | SSE policy: wire 4 MiB, event 256 KiB, output 1 MiB, idle 30 s, total 5 min |

The remaining production `bytes_stream()` calls are the shared bounded HTTP
mechanism and the bounded SSE state machine. The `.bytes()` hits in
`targets/config.rs` and `ipc_error.rs` are bounded `str` byte iterators used to
validate a 64-character digest and an IPC code segment; neither reads an HTTP
response.

GitHub denial bodies are bounded to 64 KiB and used only for typed
classification; their text is not retained in public errors or PAT test
results. The Marketplace direct downloader is absent and was not recreated.

## Bounded file and remote reads

| Call site | Final boundary |
| --- | --- |
| Central main and arbitrary skill files | Local opened-file `take(limit + 1)` and SSH/WSL `read_file_bounded`, 1 MiB; arbitrary remote containment runs first |
| Local scanner frontmatter | shared bounded text reader, 1 MiB; unreadable/oversized/invalid candidates are skipped |
| Remote scanner frontmatter | four-file process chunks; per-file `wc -c` + `dd(1 MiB + 1)`, aggregate Standard process cap |
| AI tagging filesystem fallback | shared bounded text reader, 1 MiB; in-row content is borrowed directly |
| GitHub remote plugin/candidate manifests | `read_file_bounded`, 1 MiB |
| GitHub remote snapshot repository file | `read_file_bounded` using the signed expected byte length; integrity is rechecked |
| Central update hash fallback | `read_file_bounded`, 32 MiB archive-entry cap and matching `cap + 1` supervisor policy |

## Reviewed remaining exemptions

| Call site | Reason |
| --- | --- |
| `ipc_codegen.rs`, `lib.rs` | build/test tooling reads repository-owned source/generated files |
| `bin/release-signature-verifier.rs` | operator-selected updater signature in a dedicated CLI |
| `logging.rs` | bounded rotated app-owned logs explicitly opened/exported by the user |
| `secrets/system.rs` | app-owned protected fallback payload bounded by secret-store write/decode policy |
| `services/central_operation/**` | app-owned recovery marker/manifest with write-time schema and recovery integrity policy |
| `services/local_archive_import/import.rs`, `services/github_import/import.rs` | files already admitted by archive/snapshot entry and aggregate budgets |
| `services/scanner/claude_plugin.rs`, `services/obsidian/query.rs` | app-owned plugin/registry records, not skill text |
| `services/usage/**` and generic target `read_file` callers | explicit history ingestion with separate potentially-large domain semantics |

## Final search evidence

```text
rg -n --glob '*.rs' '\.(text|bytes|bytes_stream)\(\)|read_to_string|\.read_file\(' src-tauri/src
rg -n --glob '*.rs' '&[^\n]*\.\.[^\n]*\]' src-tauri/src/services/ai_provider src-tauri/src/services/ai_tagging
```

No unexplained task-scope response `.text()`, response `.bytes()`,
`read_to_string`, or generic remote `read_file` hit remains. Remaining hits
are bounded mechanisms or the reviewed exemptions above. The string slices
reported by the second search use parser-derived UTF-8 indices or byte-buffer
indices; no fixed/dynamic byte offset slices a Rust `str`.
