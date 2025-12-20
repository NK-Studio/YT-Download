import SwiftUI

struct ContentView: View {
    @StateObject private var env = EnvironmentChecker()
    @StateObject private var downloader = YoutubeDownloader() // 다운로더 인스턴스
    
    @State private var urlInput = ""
    @State private var isLoadingFormats = false
    @State private var formats: [VideoFormat] = []
    @State private var metadata: VideoMetadata?
    @State private var errorMessage: String? // 에러 메시지 표시용
    
    var body: some View {
        ZStack {
            VStack(spacing: 0) {
                headerSection
                
                if env.isChecking {
                    ProgressView("시스템 구성 확인 중...").frame(maxHeight: .infinity)
                } else if !env.isYtdlpInstalled || !env.isFfmpegInstalled {
                    errorInstallView
                } else {
                    VStack(spacing: 0) {
                        thumbnailPreviewSection
                        
                        if downloader.isDownloading {
                            downloadingView // 다운로드 중일 때 보여줄 뷰
                        } else {
                            mainDownloadView // 리스트 영역
                        }
                    }
                }
            }
            .blur(radius: downloader.isDownloading ? 2 : 0) // 다운로드 중 뒤쪽 흐리게
            .disabled(downloader.isDownloading) // 다운로드 중 입력 막기
            
            // 다운로드 중 오버레이 (선택사항)
            if downloader.isDownloading {
                downloadProgressOverlay
            }
        }
        .background(Color.gray.opacity(0.05))
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
                Circle()
                    .fill(env.isYtdlpInstalled ? Color.green : Color.red)
                    .frame(width: 8, height: 8)
                    .help("yt-dlp 설치 상태")
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
                                    .scaleEffect(1.0)
                                    .animation(
                                        Animation.easeInOut(duration: 0.6)
                                            .repeatForever()
                                            .delay(Double(index) * 0.2),
                                        value: isLoadingFormats
                                    )
                                    .onAppear {
                                        withAnimation {
                                            // 애니메이션 트리거용
                                        }
                                    }
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
                    withAnimation {
                        downloader.isDownloading = false
                        // 완료 후 초기화 하고 싶다면 아래 주석 해제
                        // urlInput = ""
                        // formats = []
                        // metadata = nil
                    }
                }
                .buttonStyle(.borderedProminent)
            }
        }
        .padding(30)
        .background(Material.regular)
        .cornerRadius(20)
        .shadow(radius: 10)
    }
    
    private var errorInstallView: some View {
        VStack(spacing: 20) {
            Image(systemName: "exclamationmark.triangle.fill").font(.system(size: 50)).foregroundColor(.red)
            Text("필수 프로그램 확인 필요").font(.headline)
            Text("터미널에서 brew install yt-dlp ffmpeg 명령어로 설치해주세요.")
                .multilineTextAlignment(.center)
                .font(.caption)
                .foregroundColor(.secondary)
                .padding()
            
            Button("다시 확인") { env.checkRequirements() }.buttonStyle(.borderedProminent)
        }
        .frame(maxHeight: .infinity)
    }
    
    // MARK: - Logic
    
    func fetchFormats() {
        guard !urlInput.isEmpty else { return }
        // 키보드 내리기
        NSApp.keyWindow?.makeFirstResponder(nil)
        
        // 즉시 로딩 상태로 변경 (애니메이션 없이)
        isLoadingFormats = true
        errorMessage = nil
        
        // 기존 데이터는 애니메이션과 함께 제거
        withAnimation {
            formats = []
            metadata = nil
        }
        
        // 실제 데이터 Fetch
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
        
        // 기본 파일명 설정 (메타데이터에서 제목 가져오기)
        if let title = metadata?.title {
            // 파일명에 사용할 수 없는 문자 제거
            let sanitizedTitle = title
                .replacingOccurrences(of: "/", with: "-")
                .replacingOccurrences(of: ":", with: "-")
                .replacingOccurrences(of: "?", with: "")
                .replacingOccurrences(of: "\"", with: "'")
                .replacingOccurrences(of: "<", with: "")
                .replacingOccurrences(of: ">", with: "")
                .replacingOccurrences(of: "|", with: "-")
            panel.nameFieldStringValue = "\(sanitizedTitle).\(format.ext)"
        }
        
        panel.begin { response in
            if response == .OK, let saveURL = panel.url {
                // 선택된 파일 경로로 다운로드 시작
                print("저장 경로: \(saveURL.path)")
                downloader.downloadVideoToFile(url: urlInput, formatId: format.id, saveURL: saveURL)
            }
        }
    }
}
