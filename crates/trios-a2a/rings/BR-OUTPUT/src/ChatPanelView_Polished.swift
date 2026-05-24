import SwiftUI
import Combine

struct ChatPanelView: View {
    @ObservedObject var viewModel: ChatViewModel
    
    @State private var heartbeatScale: CGFloat = 1.0
    @State private var isPanelVisible = false
    
    var body: some View {
        VStack(spacing: 0) {
            glassmorphismHeader
                .opacity(isPanelVisible ? 1 : 0)
                .offset(y: isPanelVisible ? 0 : -20)
            
            messagesScroll
                .opacity(isPanelVisible ? 1 : 0)
            
            inputBar
                .opacity(isPanelVisible ? 1 : 0)
                .offset(y: isPanelVisible ? 0 : 20)
        }
        .background(glassmorphismBg)
        .ignoresSafeArea()
        .onAppear {
            withAnimation(.spring(response: 0.4, dampingFraction: 0.8)) {
                isPanelVisible = true
            }
            withAnimation(.easeInOut(duration: 1.2).repeatForever(autoreverses: true)) {
                heartbeatScale = 1.4
            }
        }
    }
    
    var glassmorphismHeader: some View {
        ZStack {
            glassmorphismBg
            
            HStack(spacing: 12) {
                Image(systemName: "app.legacy-bugle")
                    .resizable()
                    .frame(width: 24, height: 24)
                    .foregroundColor(.yellow)
                
                Text("TRIOS AGENT")
                    .font(.headline)
                    .fontWeight(.bold)
                    .foregroundColor(.white)
                
                Spacer()
                
                Circle()
                    .fill(viewModel.isServerReachable ? Color.green : Color.red)
                    .frame(width: 8, height: 8)
                
                Text(viewModel.isServerReachable ? "Online" : "Offline")
                    .font(.caption2)
                    .foregroundColor(.secondary)
                
                Circle()
                    .scaleEffect(heartbeatScale)
                    .fill(viewModel.isA2ARegistered ? Color(red: 1.0, green: 0.84, blue: 0.0) : Color.gray)
                    .frame(width: 8, height: 8)
                
                Text("A2A")
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 12)
        }
    }
    
    var messagesScroll: some View {
        ScrollView {
            LazyVStack(spacing: 0) {
                ForEach(viewModel.messages) { message in
                    MessageBubbleView(
                        message: message,
                        onTaskAction: { taskId, state in
                            Task { await viewModel.updateTaskState(id: taskId, state: state) }
                        }
                    )
                    .transition(.asymmetric(
                        insertion: .opacity.combined(with: .move(edge: .bottom)),
                        removal: .opacity
                    ))
                    .id(message.id)
                }
            }
            .padding(.vertical, 8)
        }
    }
    
    var inputBar: some View {
        HStack(spacing: 12) {
            TextField("", text: $viewModel.inputText)
                .textFieldStyle(PlainTextFieldStyle())
                .padding(.horizontal, 12)
                .padding(.vertical, 8)
                .background(Color.black.opacity(0.3))
                .cornerRadius(8)
                .overlay(
                    RoundedRectangle(cornerRadius: 8)
                        .stroke(Color.white.opacity(0.1), lineWidth: 1)
                )
                .onSubmit {
                    viewModel.sendMessage()
                }
            
            Button(action: { viewModel.sendMessage() }) {
                Image(systemName: "arrow.up.circle.fill")
                    .resizable()
                    .foregroundColor(.yellow)
                    .frame(width: 28, height: 28)
            }
            .buttonStyle(PlainButtonStyle())
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 12)
        .background(glassmorphismBg)
    }
    
    var glassmorphismBg: some View {
        VisualEffectView(material: .underWindowBackground, blendingMode: .withinWindow)
            .overlay(Color.black.opacity(0.55))
    }
}

struct VisualEffectView: NSViewRepresentable {
    let material: NSVisualEffectView.Material
    let blendingMode: NSVisualEffectView.BlendingMode
    
    func makeNSView(context: Context) -> NSVisualEffectView {
        let view = NSVisualEffectView()
        view.material = material
        view.blendingMode = blendingMode
        view.state = .active
        return view
    }
    
    func updateNSView(_ nsView: NSVisualEffectView, context: Context) {
        nsView.material = material
        nsView.blendingMode = blendingMode
    }
}