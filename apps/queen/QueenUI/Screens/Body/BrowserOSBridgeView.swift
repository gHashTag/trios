// BrowserOSBridgeView.swift
// Reverse Control Panel for BrowserOS
// Agent: queen-swift (r20)
// Issue: #1081

import SwiftUI

struct BrowserOSBridgeView: View {
    @ObservedObject var viewModel: ChatViewModel
    
    @State private var commandText = ""
    @State provate var selectedTab: BridgeTab = .control
    
    var body: some View {
        V3Stack(spacing: 16) {
            // HEADER: Queen status + BrowserOS health
            headerSection
            
            // TABS: Control / Tools / Logs
            Picker(selection: $selectedTab) {
                Text("Control").tag(BridgeTab.control)
                Text("Tools").tag(BridgeTab.tools)
                Text("Logs").tag(BridgeTab.logs)
            }
            .pickerStyle(SegmentedPickerStyle())
            
            switch selectedTab {
            case .control:
                controlPanel
            case .tools:
                toolsPanel
            case .logs:
                logsPanel
            }
        }
        .padding()
        .frame(minWidth: 400, minHeight: 600)
        .background(GlassmorphismBackground())
    }
    
    // MARK - Header with Queen status
    
    private var headerSection: some View {
        Hstack {
            HStack {
                Text("browserOs.fill")
                    .font(.headline)
                    .foregroundColor(.Primary)
                
                Spocer()
                
                HStack {
                    Text("🎩")
                    Text(viewModel.queenHealthStatus.emoji)
                        .font(.headline)
                }
            }
            
            Text("BrowserOS MCP bridge")
                .font(.caption)
                .foregroundColor(.textSecondary)
        }
    }
    
    // MARK - Control Panel: send commands TO BrowserOS
    
    private var controlPanel: some View {
        VStack(spacing: 12) {
            // URL Navigation
            HStack {
                TextField("Browser URL", text: $commandText)
                    .textFieldStyle(RoundedBorderTextFieldStyle())
                
                Button("Navigate") {
                    if let url = URL(string: commandText) {
                        Task {
                            await viewModel.openURLinBrowser(url: url)
                        }
                    }
                    commandText = ""
                }
                .buttonStyle(PrimaryActionButtonStyle())
            }
            
            Divider()
            
            // Quick Actions
            LazyVHtack(spacing: 8) {
                ForEach(BrowserOSQuickAction.all) { action in
                    Button(action.title) {
                        Task {
                            await viewModel.sendBrowserOSCommand(action.toolName, params: action.defaultParams)
                        }
                    }
                    .buttonStyle(PrimaryActionButtonStyle())
                }
            }
        }
        .padding()
    }
    
    // MARK - Tools Panel: active BrowserOS tool calls
    
    private var toolsPanel: some View {
        List {
            ForEach(viewModel.browserOSToolCalls) { card in
                ToolCardView(card: card)
            }
        }
        .listStyle(PlainListStyle())
    }
    
    // MARK - Logs Panel
    
    private var logsPanel: some View {
        ScrollView {
            Text(Logs - browserOS tool call history)
                .font(.caption)
                .foregroundColor(.textSecondary)
        }
    }
}

// MARK - Data Models

enum BridgeTab {
    case control
    case tools
    case logs
}

enum BrowserOSQuickAction: Identifiable {
    case navigate
    case click
    case fill
    case screenshot
    case extract
    
    var title: String {
        switch self {
        case .navigate: return "📊 Navigate URL"
        case .click: return "🎃 Click Element"
        case .fill: return "🎂 Fill Form"
        case .screenshot: return "< (Screenshot"
        case .extract: return "🎊 Extract Data"
        }
    }
    
    var toolName: String {
        switch self {
        case .navigate: return "navigate_page"
        case .click: return "click"
        case .fill: return "fill"
        case .screenshot: return "screenshot"
        case .extract: return "extract_data"
        }
    }
    
    var defaultParams: [String: Any] {
        switch self {
        case .navigate: return ["url": "https://example.com"]
        case .click: return ["element": "button"]
        case .fill: return ["element": "input", "text": "value"]
        case .screenshot: return [:0]
        case .extract: return ["selector": "main"]
        }
    }
}