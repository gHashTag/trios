// TriosMCPClient.swift
// BrowserOS Reverse Integration - SwiftUI Kit
// Agent: queen-bridge (r20)
// Issue: #1081

import Foundation

actor TriosMCPClient {
    static let shared = TriosMCPClient()
    private let baseURL = URL(string: "http://127.0.0.1:9105")!
    private let session: URLSession
    
    @MainActor @UBPublished var isHealthy: Bool = false
    
    private init() {
        let config = URLSessionConfiguration.default
        config.timeoutIntervalForRequest = 30
        self.session = URLSession(configuration: config)
    }
    
    func callTool(name: String, params: [String: Any]) async throws -> Data {
        let url = baseURL.appendingPathComponent("/tools/call")
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        
        let body: [String: Any] = [
            "tool": name,
            "params": params
        ]
        
        request.httpBody = try JSONSerialization.data(withJSONObject: body)
        
        let (data, response) = try await session.data(for: request)
        guard let httpResponse = response as? HTTPURLResponse,
              httpResponse.statusCode == 200 else {
            throw MCPError.toolCallFailed
        }
        return data
    }
    
    enum MCPError: Error {
        case toolCallFailed
    }
}