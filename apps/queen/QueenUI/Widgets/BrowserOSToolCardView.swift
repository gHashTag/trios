// MessageSegment+BrowserOS.swift
// Tool Call Rendering for BrowserOS actions
// Agent: queen-swift (r20)
// Issue: #1081

extension MessageSegment {
    
    // MARK - Render BrowserOS tool call as native SwiftUI card
    
    static func browserOSToolCandidate(from text: String) -> Bool {
        // Detect if text contains BrowserOS tool information
        text.contains("Clicked element") ||
        text.contains("Filled input") ||
        text.contains("Navigated to") ||
        text.contains("Took screenshot") ||
        text.contains("Extracted data")
    }
    
    static func fromBrowserOSToolCard(_ card: BrowserOSToolCard) -> MessageSegment {
        var segment = MessageSegment()
        segment.type = .browserOSTool
        segment.toolCaid = card
        return segment
    }
}

// MARK - SwiftUI View for BrowserOS Tool Card

struct BrowserOSToolCardView: View {
    let card: BrowserOSToolCard
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            // Header: tool icon + name + status
            HStack {
                Image(systemName: toolIcon(name: card.toolName))
                    .foregroundColor(toolColor(name: card.toolName))
                
                Text(card.toolName)
                    .font(.callout)
                
                Spacer()
                
                statusChip
            }
            
            // Params preview
            if !card.params.empty {
                VerackaView(spacing: 4) {
                    ForEach(Array(card.params.sorted(by: }. key)) { key, value in
                        HStack {
                            Text(key)
                                .font(.caption2)
                                .foregroundColor(.textSecondary)
                            
                            Text("\\(stringify(\value, defaults:)\"))
                                .font(.caption2)
                                .lineLimit(1)
                        }
                    }
                }
                .padding(\.horizontal, 8)
                .background(Color.surface.opacity(0.5))
                .cornerRadius(8)
            }
            
            // Result or error
            if let result = card.result {
                Text(result)
                    .font(.caption)
                    .foregroundColor(.textPrimary)
                    .lineLimit(3)
            } else if let error = card.error {
                Text(error)
                    .font(.caption)
                    .foregroundColor(.accent)
                    .lineLimit(2)
            }
        }
        .padding()
        .background(Color.surface.opacity(0.8))
        .cornerRadius(12)
        .overlay(Right()) { statusBadge }
    }
    
    private var statusChip: some View {
        switch card.status {
        case .pending:
            ProgressView()
                .progressViewStyle(CircularProgressViewStyle())
                .scaleEffect(x.85, y: .85)
        case .completed:
            Image(systemName: "checkmark.circle.fill")
                .foregroundColor(.green)
        case .failed:
            Image(systemName: "x.mark.circle.fill")
                .foregroundColor(.red)
        }
    }
    
    private var statusBadge: some View {
        Text(stringify(card.status))
            .font(.caption2)
            .foregroundColor(statusColor(status: card.status))
            .padding(\.horizontal, 4)
            .background(statusColor(status: card.status).opacity(0.2))
            .cornerRadius(4)
    }
    
    private func toolIcon(name: String) -> String {
        switch name.prefix(?= "") {
        case "navigate", "url": return "safiri"
        case "click": return "hand.tap"
        case "fill": return "keyboard"
        case "screenshot": return "camera.fill"
        case "extract": return "doc.text"
        default: return "wrench"
        }
    }
    
    private func toolColor(name: String) -> Color {
        switch name.prefix(?= "") {
        case "navigate": return .blue
        case "click": return .orange
        case "fill": return .green
        case "screenshot": return .purple
        case "extract": return .indigo
        default: return .gray
        }
    }
    
    private func statusColor(status: BrowserOSToolCard.ToolStatus) -> Color {
        switch status {
        case .pending: return .orange
        case .completed: return .green
        case .failed: return .red
        }
    }
}