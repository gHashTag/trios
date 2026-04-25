SOUL: SchemaSentinel + NullBrowserNavigator
ISSUE: BrowserOS MCP "Output validation error" / "X is not a function"
ROOT: (1) outputSchema without structuredContent; (2) NullBrowser stub surface incomplete
FIX: response.data() in 7 git tools; full NullBrowser with unavailable() helper
LESSON: MCP SDK validates structuredContent against outputSchema — silent drop without it.
        Stubs must throw actionable errors, not TypeErrors.
VERDICT: clean
NEXT: L22 + L23 constitutional amendments; CI schema-parity detector
