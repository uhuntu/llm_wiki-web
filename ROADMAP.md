# Web parity work list

Status as of 2026-11-11. A successful text workflow does not certify every format or provider.

| Area | Current evidence | Remaining scope |
|---|---|---|
| Project lifecycle | Browser export/download/import, new empty target, 27 restored files, recent-project switch, chat history, server restart; isolated null-active-state restart | Deleted/moved project recovery and broader project management edge cases |
| Formats and media | Text import and authenticated file delivery | PDF/Office/images parsing and attachment rendering, byte ranges |
| Search/graph/reviews | Original modules present | End-to-end UI and data checks |
| Watch/schedules/recovery | External watcher adapter present | Scheduled import, interruption/recovery validation and missing adapters |
| Model routing/settings | Active HTTP model | Task-specific providers and configurable request timeouts |
| Remote providers | Protocol-level checks, actual local Ollama | Credential-backed real provider validation |
| CLI/cloud/clipper/API/MCP | Partial or unsupported routes | Implement external service transports and intended Web behavior |
| Packaging/operations | Built images and standalone source, local single-user preview | First network build, sustained operation and server deployment |

Known UI issue: ingest completion may require refresh to update its badge. Upload staging retention is manual. Desktop-specific behavior requires explicit Web equivalents.
