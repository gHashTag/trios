/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * A card imported `with { type: 'file' }`: Bun gives its path, inside the
 * compiled binary or on disk.
 */
declare module '*.wasm' {
  const path: string
  export default path
}
