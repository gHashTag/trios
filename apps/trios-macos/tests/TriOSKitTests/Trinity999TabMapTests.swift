import XCTest
@testable import TriOSKit

/// The 999 petal map, checked by CI.
///
/// A standalone copy of these assertions has lived in `tests/swift/` for a
/// while, but nothing runs it: the workflow compiles four other standalone
/// tests and not this one. It had been failing since `skills` was added -
/// asserting seven hosted workspaces against a map that held eight - while two
/// plan reports recorded it as passing. `Trinity999TabMap` is inside the CI
/// slice (`rings/SR-00`), so these assertions belong where `swift test` runs.
final class Trinity999TabMapTests: XCTestCase {

    func testEveryDestinationHasExactlyOneRoute() {
        // Written against the enum rather than a hard-coded count, which is
        // the drift that let the standalone copy rot unnoticed.
        XCTAssertEqual(
            Trinity999TabMap.routes.count,
            Trios999Destination.allCases.count
        )
        for destination in Trios999Destination.allCases {
            XCTAssertNotNil(
                Trinity999TabMap.route(for: destination),
                "no route for \(destination.rawValue)"
            )
        }
    }

    func testTheMapIsInternallyValid() {
        XCTAssertTrue(Trinity999TabMap.isValid)
        XCTAssertEqual(Trinity999TabMap.petalCount, 27)
    }

    func testPetalsAndShortcutsAreUnique() {
        let petals = Trinity999TabMap.routes.map(\.petalIndex)
        let shortcuts = Trinity999TabMap.routes.map(\.keyboardShortcut)
        XCTAssertEqual(Set(petals).count, petals.count, "two workspaces share a petal")
        XCTAssertEqual(Set(shortcuts).count, shortcuts.count, "two workspaces share a shortcut")
    }

    func testEveryPetalIsInsideTheTriangle() {
        for route in Trinity999TabMap.routes {
            XCTAssertTrue(
                (0..<Trinity999TabMap.petalCount).contains(route.petalIndex),
                "\(route.destination.rawValue) sits outside the 27 petals"
            )
            XCTAssertTrue(
                (1...9).contains(route.keyboardShortcut),
                "\(route.destination.rawValue) has an unreachable shortcut"
            )
        }
    }

    func testTheHiveTookTheOnlyGapLeft() throws {
        let hive = try XCTUnwrap(Trinity999TabMap.route(for: .hive))
        XCTAssertEqual(hive.petalIndex, 15)
        XCTAssertEqual(hive.keyboardShortcut, 7)
        XCTAssertEqual(hive.realm, .materiya)
    }

    func testUnassignedPetalsKeepTheirCanonicalQueenScreens() {
        // A petal with no trios route falls through to the embedded Queen
        // surface. Claiming one silently would replace a screen the operator
        // still expects to find there.
        XCTAssertNil(Trinity999TabMap.route(forPetal: 12))
        XCTAssertNil(Trinity999TabMap.route(forPetal: 26))
    }

    func testLookupByPetalAgreesWithLookupByDestination() {
        for route in Trinity999TabMap.routes {
            XCTAssertEqual(
                Trinity999TabMap.route(forPetal: route.petalIndex)?.destination,
                route.destination
            )
        }
    }
}
