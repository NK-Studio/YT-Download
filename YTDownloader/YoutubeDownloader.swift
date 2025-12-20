import Foundation

class YoutubeDownloader: ObservableObject {
    @Published var downloadProgress: Double = 0.0
    @Published var isDownloading = false
    @Published var currentStatus: String = ""
    
    // yt-dlp 실행 경로 (EnvironmentChecker에서 찾은 경로를 쓰거나, Homebrew 기본 경로 사용)
    private let ytdlpPath = "/opt/homebrew/bin/yt-dlp" // Intel Mac은 /usr/local/bin/yt-dlp 일 수 있음
    private let ffmpegPath = "/opt/homebrew/bin/ffmpeg" // Intel Mac은 /usr/local/bin/ffmpeg 일 수 있음
    
    // 1. 영상 정보 가져오기 (JSON 파싱)
    func fetchVideoInfo(url: String, completion: @escaping (Result<(VideoMetadata, [VideoFormat]), Error>) -> Void) {
        let task = Process()
        task.executableURL = URL(fileURLWithPath: ytdlpPath)
        task.arguments = ["-j", url] // JSON 덤프
        
        let pipe = Pipe()
        task.standardOutput = pipe
        
        do {
            try task.run()
            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            
            // JSON 파싱
            let decoder = JSONDecoder()
            let info = try decoder.decode(YtdlpResponse.self, from: data)
            
            let metadata = VideoMetadata(title: info.title, thumbnailUrl: info.thumbnail)
            
            // 포맷 필터링 (mp4이면서 오디오가 있거나, 사이즈가 있는 것들 위주로)
            let formats: [VideoFormat] = info.formats.compactMap { f in
                // 너무 작은 썸네일 전용 포맷 등은 제외
                guard let filesize = f.filesize else { return nil }
                
                let res = (f.height != nil) ? "\(f.height!)p" : "Audio"
                let sizeMB = Double(filesize) / 1024 / 1024
                let sizeStr = String(format: "%.1f MB", sizeMB)
                let codecName = f.vcodec ?? "none"
                
                return VideoFormat(
                    id: f.format_id,
                    ext: f.ext,
                    resolution: res,
                    fileSize: sizeStr,
                    codec: codecName,
                    vcodec: codecName
                )
            }.reversed() // 고화질이 보통 뒤에 있어서 역순 정렬
            
            completion(.success((metadata, formats)))
            
        } catch {
            completion(.failure(error))
        }
    }
    
    // 2. 다운로드 실행 (폴더 선택 방식 - 기존)
    func downloadVideo(url: String, formatId: String, savePath: URL) {
        DispatchQueue.main.async {
            self.isDownloading = true
            self.downloadProgress = 0.0
            self.currentStatus = "다운로드 시작..."
        }
        
        let task = Process()
        task.executableURL = URL(fileURLWithPath: ytdlpPath)
        
        // 다운로드 경로 설정 (파일명 템플릿 지정)
        let outputTemplate = savePath.appendingPathComponent("%(title)s.%(ext)s").path
        
        // 진행률 파싱을 위해 output 형식을 지정
        task.arguments = ["-f", formatId, "-o", outputTemplate, url, "--newline"]
        
        let pipe = Pipe()
        task.standardOutput = pipe
        
        let outHandle = pipe.fileHandleForReading
        
        // 실시간 출력 읽기
        outHandle.readabilityHandler = { pipe in
            if let line = String(data: pipe.availableData, encoding: .utf8) {
                DispatchQueue.main.async {
                    self.parseProgress(line)
                }
            }
        }
        
        task.terminationHandler = { _ in
            DispatchQueue.main.async {
                self.isDownloading = false
                self.currentStatus = "다운로드 완료!"
                self.downloadProgress = 1.0
            }
        }
        
        try? task.run()
    }
    
    // 3. 다운로드 실행 (파일 경로 직접 지정 방식 - 새로 추가)
    func downloadVideoToFile(url: String, formatId: String, saveURL: URL) {
        DispatchQueue.main.async {
            self.isDownloading = true
            self.downloadProgress = 0.0
            self.currentStatus = "다운로드 시작..."
        }
        
        let task = Process()
        task.executableURL = URL(fileURLWithPath: ytdlpPath)
        
        // 사용자가 지정한 정확한 파일 경로 사용
        let outputPath = saveURL.path
        
        // 진행률 파싱을 위해 output 형식을 지정
        // -f 대신 bestvideo+bestaudio 형식으로 자동 병합 또는 formatId+bestaudio로 지정
        // --merge-output-format mp4로 최종 출력을 mp4로 통일
        // --ffmpeg-location으로 ffmpeg 경로 명시
        task.arguments = [
            "-f", "\(formatId)+bestaudio/best",  // 선택한 비디오 + 최고 오디오 병합, 없으면 best
            "-o", outputPath,
            "--merge-output-format", "mp4",      // 병합 시 mp4로 출력
            "--ffmpeg-location", ffmpegPath,     // ffmpeg 경로 명시
            url,
            "--newline"
        ]
        
        let pipe = Pipe()
        task.standardOutput = pipe
        
        let outHandle = pipe.fileHandleForReading
        
        // 실시간 출력 읽기
        outHandle.readabilityHandler = { pipe in
            if let line = String(data: pipe.availableData, encoding: .utf8) {
                DispatchQueue.main.async {
                    self.parseProgress(line)
                }
            }
        }
        
        task.terminationHandler = { _ in
            DispatchQueue.main.async {
                self.isDownloading = false
                self.currentStatus = "다운로드 완료!"
                self.downloadProgress = 1.0
            }
        }
        
        try? task.run()
    }
    
    // 출력 로그에서 % 파싱
    private func parseProgress(_ log: String) {
        // 예: [download]  45.0% of 10.00MiB at  2.00MiB/s
        if log.contains("[download]") && log.contains("%") {
            let parts = log.components(separatedBy: "%")
            if let firstPart = parts.first, let percentStr = firstPart.components(separatedBy: " ").last {
                if let percent = Double(percentStr) {
                    self.downloadProgress = percent / 100.0
                    self.currentStatus = "다운로드 중... \(Int(percent))%"
                }
            }
        }
    }
}
