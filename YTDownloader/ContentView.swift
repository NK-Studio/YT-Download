import SwiftUI

struct ContentView: View {
    @StateObject private var env = EnvironmentChecker()
    @StateObject private var downloader = YoutubeDownloader()

    @State private var urlInput = ""
    @State private var isLoadingFormats = false
    @State private var formats: [VideoFormat] = []
    @State private var metadata: VideoMetadata?
    @State private var errorMessage: String?
    @State private var setupAnimating = false
    @State private var updateAnimating = false

    var body: some View {
        ZStack {
            VStack(spacing: 0) {
                headerSection

                if env.isChecking {
                    setupView
                } else if !env.isYtdlpInstalled {
                    errorInstallView
                } else {
                    VStack(spacing: 0) {
                        thumbnailPreviewSection

                        if downloader.isDownloading {
                            downloadingView
                        } else {
                            mainDownloadView
                        }
                    }
                }
            }
            .blur(radius: downloader.isDownloading ? 2 : 0)
            .disabled(downloader.isDownloading)

            if downloader.isDownloading {
                downloadProgressOverlay
            }

            if env.isUpdating {
                updateProgressOverlay
            }
        }
        .background(Color.gray.opacity(0.05))
        .onChange(of: env.ytdlpPath) { newPath in
            downloader.ytdlpPath = newPath.path
        }
        .onChange(of: env.ffmpegPath) { newPath in
            downloader.ffmpegPath = newPath.path
        }
        .onAppear {
            downloader.ytdlpPath = env.ytdlpPath.path
            downloader.ffmpegPath = env.ffmpegPath.path
        }
    }

    // MARK: - Components

    private var thumbnailPreviewSection: some View {
        Group {
            if let metadata = metadata {
                VStack(alignment: .leading, spacing: 8) {
                    AsyncImage(url: URL(string: metadata.thumbnailUrl)) { image in
                        image.resizable()
                            .aspectRatio(16/9, contentMode: .fill)
                    } placeholder: {
                        Rectangle().fill(Color.black.opacity(0.1))
                            .aspectRatio(16/9, contentMode: .fit)
                            .overlay(ProgressView())
                    }
                    .cornerRadius(12)
                    .clipped()

                    Text(metadata.title)
                        .font(.headline)
                        .lineLimit(2)
                        .padding(.horizontal, 4)
                }
                .padding()
                .transition(.scale.combined(with: .opacity))
            }
        }
    }

    private var headerSection: some View {
        VStack(spacing: 12) {
            HStack {
                Text("NUPY YT Downloader").font(.caption).fontWeight(.semibold)
                Spacer()

                // 버전 배지
                if !env.ytdlpVersion.isEmpty {
                    Text("v\(env.ytdlpVersion)")
                        .font(.caption2)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(Color.blue.opacity(0.15))
                        .foregroundColor(.blue)
                        .cornerRadius(4)
                }

                // 업데이트 버튼
                if env.updateAvailable && !env.isUpdating {
                    Button {
                        Task { await env.updatePlugins() }
                    } label: {
                        Label("업데이트", systemImage: "arrow.down.circle")
                            .font(.caption)
                            .foregroundColor(.orange)
                    }
                    .buttonStyle(.plain)
                    .help("최신 버전(\(env.latestVersion))으로 업데이트")
                }

                Circle()
                    .fill(env.isYtdlpInstalled ? Color.green : Color.red)
                    .frame(width: 8, height: 8)
                    .help("플러그인 상태")
            }
            HStack {
                Image(systemName: "link").foregroundColor(.blue)
                TextField("유튜브 링크 (cmd+v)", text: $urlInput)
                    .textFieldStyle(.plain)
                    .onSubmit { fetchFormats() }
                    .disabled(isLoadingFormats)

                if isLoadingFormats {
                    ProgressView()
                        .scaleEffect(0.7)
                        .frame(width: 20, height: 20)
                }
            }
            .padding(10)
            .background(Color.white.opacity(0.1))
            .cornerRadius(8)
            .overlay(RoundedRectangle(cornerRadius: 8).stroke(isLoadingFormats ? Color.blue.opacity(0.5) : Color.gray.opacity(0.2)))

            if let error = errorMessage {
                Text(error)
                    .foregroundColor(.red)
                    .font(.caption)
            }
        }
        .padding()
        .background(Color.white.opacity(0.05))
    }

    private var mainDownloadView: some View {
        ScrollView {
            VStack(spacing: 10) {
                if isLoadingFormats {
                    VStack(spacing: 20) {
                        ProgressView()
                            .scaleEffect(1.5)
                            .progressViewStyle(.circular)

                        Text("영상 정보 분석 중...")
                            .font(.subheadline)
                            .foregroundColor(.secondary)

                        HStack(spacing: 4) {
                            ForEach(0..<3) { index in
                                Circle()
                                    .fill(Color.blue)
                                    .frame(width: 8, height: 8)
                                    .opacity(0.3)
                                    .animation(
                                        Animation.easeInOut(duration: 0.6)
                                            .repeatForever()
                                            .delay(Double(index) * 0.2),
                                        value: isLoadingFormats
                                    )
                            }
                        }
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.top, 60)

                } else if formats.isEmpty && metadata != nil {
                    Text("다운로드 가능한 포맷이 없습니다.")
                        .foregroundColor(.secondary)
                        .padding(.top, 20)
                } else {
                    ForEach(formats) { format in
                        Button(action: { saveFileDialog(format) }) {
                            HStack {
                                VStack(alignment: .leading) {
                                    Text(format.resolution).font(.headline)
                                    Text("\(format.ext) • \(format.codecTag)").font(.caption).foregroundColor(.secondary)
                                }
                                Spacer()
                                Text(format.fileSize).font(.subheadline).foregroundColor(Color(red: 0, green: 183/255, blue: 255/255))
                                Image(systemName: "arrow.down.circle.fill")
                                    .font(.title2)
                                    .foregroundColor(Color(red: 0, green: 183/255, blue: 255/255))
                            }
                            .padding()
                            .background(Color.white.opacity(0.35))
                            .cornerRadius(10)
                            .shadow(color: .black.opacity(0.1), radius: 3, x: 0, y: 1)
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
            .padding()
        }
    }

    private var downloadingView: some View {
        VStack {
            Spacer()
            Text("다운로드가 백그라운드에서 진행중입니다.")
                .foregroundColor(.secondary)
            Spacer()
        }
    }

    private var downloadProgressOverlay: some View {
        VStack(spacing: 15) {
            Text(downloader.currentStatus)
                .font(.headline)

            ProgressView(value: downloader.downloadProgress)
                .progressViewStyle(.linear)
                .frame(width: 200)

            if downloader.downloadProgress >= 1.0 {
                Button("완료") {
                    withAnimation { downloader.isDownloading = false }
                }
                .buttonStyle(.borderedProminent)
            }
        }
        .padding(30)
        .background(Material.regular)
        .cornerRadius(20)
        .shadow(radius: 10)
    }

    // 플러그인 업데이트 진행 오버레이
    private var updateProgressOverlay: some View {
        ZStack {
            // 배경 디밍
            Color.black.opacity(0.25)
                .ignoresSafeArea()

            VStack(spacing: 24) {
                ZStack {
                    // 회전하는 외곽 그라디언트 링
                    Circle()
                        .trim(from: 0, to: 0.75)
                        .stroke(
                            AngularGradient(
                                colors: [
                                    Color(red: 0, green: 183/255, blue: 255/255).opacity(0.0),
                                    Color(red: 0, green: 183/255, blue: 255/255),
                                    .blue
                                ],
                                center: .center
                            ),
                            style: StrokeStyle(lineWidth: 3, lineCap: .round)
                        )
                        .frame(width: 120, height: 120)
                        .rotationEffect(.degrees(updateAnimating ? 360 : 0))
                        .animation(
                            .linear(duration: 1.6).repeatForever(autoreverses: false),
                            value: updateAnimating
                        )

                    // 트랙 (배경 원)
                    Circle()
                        .stroke(Color.gray.opacity(0.15), lineWidth: 9)
                        .frame(width: 96, height: 96)

                    // 진행 원
                    Circle()
                        .trim(from: 0, to: env.updateProgress)
                        .stroke(
                            LinearGradient(
                                colors: [Color(red: 0, green: 183/255, blue: 255/255), .blue],
                                startPoint: .top, endPoint: .bottom
                            ),
                            style: StrokeStyle(lineWidth: 9, lineCap: .round)
                        )
                        .frame(width: 96, height: 96)
                        .rotationEffect(.degrees(-90))
                        .shadow(color: Color(red: 0, green: 183/255, blue: 255/255).opacity(0.5), radius: 6)
                        .animation(.easeOut(duration: 0.25), value: env.updateProgress)

                    // 가운데 퍼센트
                    VStack(spacing: -2) {
                        Text("\(Int(env.updateProgress * 100))")
                            .font(.system(size: 30, weight: .bold, design: .rounded))
                            .contentTransition(.numericText())
                            .animation(.easeOut, value: env.updateProgress)
                        Text("%")
                            .font(.system(size: 11, weight: .medium, design: .rounded))
                            .foregroundColor(.secondary)
                    }
                }
                .frame(width: 130, height: 130)

                VStack(spacing: 6) {
                    HStack(spacing: 6) {
                        Image(systemName: "arrow.down.circle.fill")
                            .foregroundStyle(
                                LinearGradient(
                                    colors: [Color(red: 0, green: 183/255, blue: 255/255), .blue],
                                    startPoint: .top, endPoint: .bottom
                                )
                            )
                        Text("플러그인 업데이트 중")
                            .font(.headline)
                    }

                    Text("잠시만 기다려주세요...")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            .padding(.horizontal, 40)
            .padding(.vertical, 32)
            .background(
                RoundedRectangle(cornerRadius: 24)
                    .fill(Material.ultraThick)
                    .overlay(
                        RoundedRectangle(cornerRadius: 24)
                            .stroke(Color.white.opacity(0.12), lineWidth: 1)
                    )
            )
            .shadow(color: .black.opacity(0.3), radius: 24, x: 0, y: 10)
        }
        .transition(.opacity.combined(with: .scale(scale: 0.95)))
        .onAppear { updateAnimating = true }
        .onDisappear { updateAnimating = false }
    }

    // 초기 설정 화면 (애니메이션 + 단계 표시)
    private var setupView: some View {
        VStack(spacing: 28) {
            Spacer()

            ZStack {
                // 바깥 펄스 링
                ForEach(0..<3) { i in
                    Circle()
                        .stroke(
                            LinearGradient(
                                colors: [Color(red: 0, green: 183/255, blue: 255/255).opacity(0.5), .blue.opacity(0)],
                                startPoint: .top, endPoint: .bottom
                            ),
                            lineWidth: 2
                        )
                        .frame(width: 90, height: 90)
                        .scaleEffect(setupAnimating ? 1.6 : 0.8)
                        .opacity(setupAnimating ? 0 : 0.8)
                        .animation(
                            .easeOut(duration: 1.8)
                                .repeatForever(autoreverses: false)
                                .delay(Double(i) * 0.6),
                            value: setupAnimating
                        )
                }

                // 메인 아이콘
                Circle()
                    .fill(
                        LinearGradient(
                            colors: [Color(red: 0, green: 183/255, blue: 255/255), .blue],
                            startPoint: .topLeading, endPoint: .bottomTrailing
                        )
                    )
                    .frame(width: 80, height: 80)
                    .shadow(color: Color(red: 0, green: 183/255, blue: 255/255).opacity(0.4), radius: 12, x: 0, y: 4)

                Image(systemName: "sparkles")
                    .font(.system(size: 32, weight: .medium))
                    .foregroundColor(.white)
                    .symbolEffect(.variableColor.iterative, options: .repeating, value: setupAnimating)
            }
            .frame(height: 100)

            VStack(spacing: 10) {
                Text("초기 세팅 중")
                    .font(.title3)
                    .fontWeight(.semibold)

                Text(env.setupStatus)
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .id(env.setupStatus)
                    .transition(.opacity.combined(with: .move(edge: .bottom)))
                    .animation(.easeInOut(duration: 0.3), value: env.setupStatus)
            }

            HStack(spacing: 8) {
                ForEach(0..<3) { i in
                    Circle()
                        .fill(Color(red: 0, green: 183/255, blue: 255/255))
                        .frame(width: 8, height: 8)
                        .scaleEffect(setupAnimating ? 1.0 : 0.4)
                        .opacity(setupAnimating ? 1.0 : 0.3)
                        .animation(
                            .easeInOut(duration: 0.6)
                                .repeatForever()
                                .delay(Double(i) * 0.15),
                            value: setupAnimating
                        )
                }
            }

            Spacer()

            Text("처음 실행 시 필요한 플러그인을 설치합니다")
                .font(.caption2)
                .foregroundColor(.secondary.opacity(0.6))
                .padding(.bottom, 20)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .onAppear { setupAnimating = true }
    }

    // yt-dlp 미설치 시 안내 (다운로드 버튼 제공)
    private var errorInstallView: some View {
        VStack(spacing: 20) {
            Image(systemName: "exclamationmark.triangle.fill")
                .font(.system(size: 50))
                .foregroundColor(.orange)

            Text("필수 플러그인이 필요합니다")
                .font(.headline)

            Text("최신 버전을 다운로드합니다.\n인터넷 연결이 필요합니다.")
                .multilineTextAlignment(.center)
                .font(.caption)
                .foregroundColor(.secondary)
                .padding(.horizontal)

            if env.isUpdating {
                VStack(spacing: 8) {
                    ProgressView(value: env.updateProgress)
                        .progressViewStyle(.linear)
                        .frame(width: 200)
                    Text("다운로드 중... \(Int(env.updateProgress * 100))%")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
            } else {
                Button {
                    Task { await env.updatePlugins() }
                } label: {
                    Label("플러그인 설치", systemImage: "arrow.down.circle.fill")
                }
                .buttonStyle(.borderedProminent)
            }

            Button("다시 확인") { env.checkRequirements() }
                .buttonStyle(.bordered)
                .disabled(env.isUpdating)
        }
        .frame(maxHeight: .infinity)
    }

    // MARK: - Logic

    func fetchFormats() {
        guard !urlInput.isEmpty else { return }
        NSApp.keyWindow?.makeFirstResponder(nil)

        isLoadingFormats = true
        errorMessage = nil

        withAnimation {
            formats = []
            metadata = nil
        }

        downloader.fetchVideoInfo(url: urlInput) { result in
            DispatchQueue.main.async {
                self.isLoadingFormats = false
                switch result {
                case .success(let (meta, fmt)):
                    withAnimation {
                        self.metadata = meta
                        self.formats = fmt
                    }
                case .failure(let error):
                    self.errorMessage = "정보를 가져오는데 실패했습니다.\n\(error.localizedDescription)"
                }
            }
        }
    }

    func saveFileDialog(_ format: VideoFormat) {
        let panel = NSSavePanel()
        panel.allowedContentTypes = [.init(filenameExtension: format.ext) ?? .data]
        panel.canCreateDirectories = true
        panel.isExtensionHidden = false
        panel.allowsOtherFileTypes = false
        panel.title = "영상을 저장할 위치를 선택하세요"
        panel.message = "파일 이름을 입력하고 저장 위치를 선택하세요"
        panel.prompt = "저장"

        if let title = metadata?.title {
            let sanitized = title
                .replacingOccurrences(of: "/", with: "-")
                .replacingOccurrences(of: ":", with: "-")
                .replacingOccurrences(of: "?", with: "")
                .replacingOccurrences(of: "\"", with: "'")
                .replacingOccurrences(of: "<", with: "")
                .replacingOccurrences(of: ">", with: "")
                .replacingOccurrences(of: "|", with: "-")
            panel.nameFieldStringValue = "\(sanitized).\(format.ext)"
        }

        panel.begin { response in
            if response == .OK, let saveURL = panel.url {
                downloader.downloadVideoToFile(url: urlInput, formatId: format.id, saveURL: saveURL)
            }
        }
    }
}
