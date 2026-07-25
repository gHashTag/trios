// swift-tools-version:5.9
import PackageDescription

// TriOSKit — the CI-testable library slice of the macOS `trios` app.
//
// Migrated from the browseros repo root (Волна 5 consolidation). The original
// package lived at the browseros *root* with `path: "trios"`; here the whole
// app tree is the package root (`apps/trios-macos/`), so the target path is
// "." and the sources / tests paths drop the `trios/` prefix.
//
// This is a compile/test-only slice: no GUI, no signing, no full app bundle.
// `build.sh` (full .app) still depends on the private Trinity Queen SwiftPM
// package and is intentionally out of CI scope — see .github/workflows.
let package = Package(
    name: "TriOS",
    platforms: [.macOS(.v14)],
    products: [
        .library(name: "TriOSKit", targets: ["TriOSKit"]),
    ],
    targets: [
        .target(
            name: "TriOSKit",
            path: ".",
            sources: [
                "rings/SR-00",
                "rings/SR-01",
                "rings/SR-02",
                "BR-OUTPUT/ProjectPaths.swift",
                "BR-OUTPUT/TriosTheme.swift",
                "BR-OUTPUT/GitHubModels.swift",
                "BR-OUTPUT/GitHubAPIClient.swift",
                "BR-OUTPUT/QueenStatusViewModel.swift",
                "BR-OUTPUT/A2AMessageRouter.swift",
                "BR-OUTPUT/ChatLogic.swift",
                "BR-OUTPUT/CladeGuard.swift",
            ]
        ),
        .testTarget(
            name: "TriOSKitTests",
            dependencies: ["TriOSKit"],
            path: "tests/TriOSKitTests"
        ),
    ]
)
