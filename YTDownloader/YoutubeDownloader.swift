import Foundation

class YoutubeDownloader: ObservableObject {
    @Published var downloadProgress: Double = 0.0
    @Published var isDownloading = false
    @Published var currentStatus: String = ""

    var ytdlpPath: String = ""
    var ffmpegPath: String = ""

    func fetchVideoInfo(url: String, completion: @escaping (Result<(VideoMetadata, [VideoFormat]), Error>) -> Void) {
        guard !ytdlpPath.isEmpty else {
            completion(.failure(DownloaderError.ytdlpNotFound))
            return
        }

        let task = Process()
        task.executableURL = URL(fileURLWithPath: ytdlpPath)
        task.arguments = ["-j", url]

        let pipe = Pipe()
        task.standardOutput = pipe
        task.standardError = Pipe()

        do {
            try task.run()
            let data = pipe.fileHandleForReading.readDataToEndOfFile()

            let decoder = JSONDecoder()
            let info = try decoder.decode(YtdlpResponse.self, from: data)

            let metadata = VideoMetadata(title: info.title, thumbnailUrl: info.thumbnail)

            let formats: [VideoFormat] = info.formats.compactMap { f in
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
            }.reversed()

            completion(.success((metadata, formats)))
        } catch {
            completion(.failure(error))
        }
    }

    func downloadVideoToFile(url: String, formatId: String, saveURL: URL) {
        guard !ytdlpPath.isEmpty else { return }

        DispatchQueue.main.async {
            self.isDownloading = true
            self.downloadProgress = 0.0
            self.currentStatus = "다운로드 시작..."
        }

        let task = Process()
        task.executableURL = URL(fileURLWithPath: ytdlpPath)

        let outputPath = saveURL.path
        var args: [String] = [
            "-f", "\(formatId)+bestaudio/best",
            "-o", outputPath,
            "--merge-output-format", "mp4"
        ]
        if !ffmpegPath.isEmpty, FileManager.default.fileExists(atPath: ffmpegPath) {
            args.append(contentsOf: ["--ffmpeg-location", ffmpegPath])
        }
        args.append(contentsOf: [url, "--newline"])
        task.arguments = args

        let pipe = Pipe()
        task.standardOutput = pipe
        let outHandle = pipe.fileHandleForReading

        outHandle.readabilityHandler = { handle in
            if let line = String(data: handle.availableData, encoding: .utf8) {
                DispatchQueue.main.async { self.parseProgress(line) }
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

    private func parseProgress(_ log: String) {
        if log.contains("[download]") && log.contains("%") {
            let parts = log.components(separatedBy: "%")
            if let firstPart = parts.first,
               let percentStr = firstPart.components(separatedBy: " ").last,
               let percent = Double(percentStr) {
                self.downloadProgress = percent / 100.0
                self.currentStatus = "다운로드 중... \(Int(percent))%"
            }
        }
    }

    enum DownloaderError: LocalizedError {
        case ytdlpNotFound
        var errorDescription: String? { "yt-dlp를 찾을 수 없습니다." }
    }
}
