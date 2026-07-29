import Foundation
import XCTest
@testable import TriOSKit

// Shared network double, extracted out of SSETransportTests.
//
// It lived inside that file, so excluding the file from the target took
// MockURLProtocol with it and broke three tests that had nothing wrong with
// them. A helper used by more than one suite does not belong inside any one
// of them.

/// URLProtocol subclass that intercepts requests and returns a canned response.
final class MockURLProtocol: URLProtocol {
    static var requestHandler: ((URLRequest) throws -> (HTTPURLResponse, Data))?
    static var chunkHandler: ((URLRequest) throws -> (HTTPURLResponse, [Data]))?

    override class func canInit(with request: URLRequest) -> Bool {
        return true
    }

    override class func canonicalRequest(for request: URLRequest) -> URLRequest {
        return request
    }

    override func startLoading() {
        if let chunkHandler = MockURLProtocol.chunkHandler {
            do {
                let (response, chunks) = try chunkHandler(request)
                client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
                for chunk in chunks {
                    client?.urlProtocol(self, didLoad: chunk)
                }
                client?.urlProtocolDidFinishLoading(self)
            } catch {
                client?.urlProtocol(self, didFailWithError: error)
            }
            return
        }
        guard let handler = MockURLProtocol.requestHandler else {
            fatalError("MockURLProtocol.requestHandler is not set")
        }
        do {
            let (response, data) = try handler(request)
            client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
            client?.urlProtocol(self, didLoad: data)
            client?.urlProtocolDidFinishLoading(self)
        } catch {
            client?.urlProtocol(self, didFailWithError: error)
        }
    }

    override func stopLoading() {}
}

extension URLSessionConfiguration {
    static func mockProtocolConfiguration() -> URLSessionConfiguration {
        let config = URLSessionConfiguration.default
        config.protocolClasses = [MockURLProtocol.self]
        config.timeoutIntervalForRequest = 120
        config.timeoutIntervalForResource = 600
        config.httpShouldSetCookies = false
        return config
    }
}
