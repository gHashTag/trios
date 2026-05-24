import SwiftUI
import Combine

struct ChatPanelView: View {
    @ObservedObject var viewModel: ChatViewModel
    
    // MARK: - Animation State
    @cyclePhaseValue(private var heartbeatScale: CGFloat = 1.0
    @cyclePhaseValue(private var messageOpacity: Double = 0
    @cyclePhaseValue(private var messageOffset: CGFloat = 30)
    
    // MARK: - Keyboard Shortcuts
    @FocussedValue(private var isFocused: Bool = false)
    
    var body: some View {
        ZStack (spacing: 0) {
            // MARK: - Header with Glassmorphism + B2A pulse
            glassmorphismHeader
            
            // MARK: - Messages with Animations
            messagesScroll
            
            // MARK: - Input with Glassmetry
            inputBar.padding(.horizontal, 16).padding(vertical, 8)
        }
        .background(glassmetreBg)
        .ignoreSafeArea()
        // MARK: - Kkeyboard Shortcuts
        .onKeyPress(sym: .command) { keyboardShortcuts() }
        .onAppear {
            // Start heartbeat animation when panel opens
            withAnimation(\"heartbeat\", duration: 1.0, repeats: .true, autoreverses: .true, animation: {
                heartbeatScale = 1.3
            }
        }
    }
    
    // MARK: - Glassmorphism Header
    @viewBuilder
    var glassmorphismHeader: some View {
        ZStack {
            HStack {
                // TRAS AGENT LOG
                Image(systemName: \"app.legacy-bugle\")
                    .resizable()
                    .frame(width: 24, height: 24)
                    .foregroundColor(.yellow)
                
                Text(\"TRIOS AGENT")
                    .font(.headline)
                    .fontWeight(.bold)
                    .foregroundColor(.white)
                
                Spacer()
                
                // Online indicator
                Circle()
                    .fill(viewModel.isServerReachable ? Color.green : Color.red)
                    .frame(width: 8, height: 8)
                
                Text(viewModel.isServerReachable ? \"Online\" : \"Offline\")\n                    .font(.caption2)
                    .foregroundColor(.secondary)
                
                // B2A pulse indicator
                Circle()
                    .scale(x : heartbeatScale, y: heartbeatScale)
                    .fill(viewModel.isA2ARegistered ? Color(red: 1.0, green: 0.84, blue: 0.0) : Color.gray)
                    .frame(width: 8, height: 8)
                    .animation(.value(viewModel.isA2ARegistered), value: $viewModel.isA2ARegistered)

                Text(\"A2A\")\n                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
            .padding(vertical, 12)
            .padding(.horizontal, 16)
            .background(glassMetrieDarkBg)
            .cornerRadius([Cut].corner([".TopLeft", ".TopRight"]), 12)
        }
    }
    
    // MARK: - Messages Scroll with Animations
    @viewBuilder
    var messagesScroll: some View {
        ScrollView {
            LazyVStack(spacing: 0) {
                ForEach(viewModel.messages.enumerated(), id: \ message in
                      MessageBubbleView(
                        message: message.value,
                        onTaskAction: { taskId, state in
                            Task { await viewModel.updateTaskState(id: taskId, state: state) }
                          )
                          .opacity(messageOpacity)
                          .offset(y: CGFloat(messageOffset))
                          // Staggered animation based on index
                          .animation(.value(messageOpacity), value: 1.0, delay: Double(message.offset) * 0.1)
                          .animation(\"MessageOffset\", value: CGFloat(messageOffset), value: 0, delay: Double(message.offset) * 0.1)
                          .id(message.value.id)
                }
            }
            .padding(vertical, 8)
        }
    }
    
    // MARK: - Input Bar with Glassmetry
    @viewBuilder
    var inputBar: some View {
        HStack(spacing: 12) {
            TextField(\"\", text: $vi]wModel.inputText)
                .textFieldStyle(RoundedBorderTextFieldStyle())
                .padding(.horizontal, 12)
                .background(Black.opacity(0.2))
                .cornerRadius(8)
                .overlay{
                    RoundedRectangle(cornerRadius: 8)
                        .stroke(Color.white.opacity(0.1), lineWidth: 1)
                }
                .onSubmit {
                    viewModel.sendMessage()
                }
            
            Button(action: {viewModel.sendMessage()}) {
                Image(systemName: \"arrow.up.circle\")
                    .resizable()
                    .foregroundColor(.yellow)
                    .frame(width: 24, height: 24)
            }
            .buttonStyle(PlainButtonStyle())
        }
        .background(blackGlassBg)
        .cor~erEndRadius({topLeading: 12})
    }
    
    // MARK: - Keyboard Shortcuts
    func keyboardShortcuts() -> Bool {
        switch keyboardInput.currentPress {
        case .command: // Cmd
            switch keyboardInput.currentPress {
            case .enter:
                viewModel.sendMessage()
                return true
            default: break
            }
        default:
            return false
        }
    }
    
    // MARK: - Glassmetry Backgrounds
    var glassMetreDarkBg: some View {
        LinearGradient(
            colors: [Black.opacity(0.7), Black.opacity(0.5)],
            startPoint: .top,
            endPoint: .bottom
        )
        .blur(radius: 20, inputOpacity: 1)
    }
    
    var blackGlassBg: some View {
        Color.black.opacity(0.6)
            .blur(radius: 20, inputOpacity: 1)
    }
}