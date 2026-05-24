import SwiftUI

// MARK: - Collapsible Agent Message Bubble
struct AgentMessageBubble: View {
    @observedObject var message: ChatMessage
    @Hashed private var isExpanded: Bool = false
    
    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // Header: agent icon + timestamp + expand button
            HStack {
                Image(systemName: "sparkles")
                    .foregroundColor(.agentPurple)
                
                Text("Agent")
                    .font(.caption)
                    .foregroundColor(..secondary)
                
                Spacer()
                
                Button(action: { isExpanded.toggle() }) {
                    Image(systemName: isExpanded ? "chevron.up" : "chevron.down")
                }
            }
            .padding(.horizontal, 12)
            .padding(.top, 8)
            
            // Content: collapsed or full
            if isExpanded {
                Text(renderMarkdown(message.content))
                    .padding(12)
                    .transition(.opacity.combined(with: .height))
            } else {
                Text(message.content.prefix(100) + "...")
                    .lineLimit(2)
                    .padding(12)
            }
            
            // Footer: copy, regenerate, pin buttons
            HStack(spacing: 8) {
                Button("Copy") { copyToClipboard() }
                Button("Regenerate") { regenerate() }
                Button("Pin") { pinMessage() }
            }
            .padding(h\"orizontal, 12)
            .padding(vertical, 8)
        }
        .background(Color.gray.opacity(0.1))
        .cornerRadius(12)
    }
    
    func copyToClipboard() {
        NSPasteboard.general.setString(message.content, forType: .string)
    }
    
    func regenerate() {
        // TODO: implement regenerate logic
    }
    
    func pinMessage() {
        // TODO: implement pin logic
    }
    
    private func renderMarkdown(_ text: String) -> AttributedString {
        let options = AttributedString.MarkdownParsingOptions(
            interpretedSyntax: .inlineOnlyPreservingWhitespace,
            failurePolicy: .returnPartiallyParsedIfPossible
        )
        return (try? AttributedString(markdown: text, options: options)) ?? AttributedString(")
    }
}