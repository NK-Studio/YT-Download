import Foundation

class EnvironmentChecker: ObservableObject {
    @Published var isYtdlpInstalled = false
    @Published var isChecking = true
    @Published var setupStatus = "준비 중..."
    @Published var ytdlpVersion = ""
    @Published var latestVersion = ""

    // ffmpeg 버전 (UI에 표시하지 않고 내부 추적용)
    @Published var ffmpegVersion = ""
    @Published var latestFfmpegVersion = ""

    @Published var isUpdating = false
    @Published var updateProgress: Double = 0

    private var ytdlpUpdateAvailable: Bool {
        !ytdlpVersion.isEmpty && !latestVersion.isEmpty && ytdlpVersion != latestVersion
    }
    // ffmpeg는 빌드별로 버전 표기가 달라 비교가 부정확하므로 배지 트리거에는 사용하지 않는다.
    // 단, 사용자가 업데이트 버튼을 누를 때 함께 갱신되도록 updatePlugins() 내부에서는 처리.
    var updateAvailable: Bool { ytdlpUpdateAvailable }

    private let githubApiUrl = "https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest"
    private let ytdlpZipUrl = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos.zip"
    private let ffmpegInfoUrl = "https://evermeet.cx/ffmpeg/info/ffmpeg/release"
    private let ffmpegZipUrl = "https://evermeet.cx/ffmpeg/getrelease/zip"

    private(set) var binDir: URL
    @Published private(set) var ytdlpPath: URL
    @Published private(set) var ffmpegPath: URL

    init() {
        let appSupport = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first!
        binDir = appSupport
            .appendingPathComponent("NKStudio")
            .appendingPathComponent("YTDownloader")
            .appendingPathComponent("bin")
        ytdlpPath = binDir.appendingPathComponent("yt-dlp_macos")
        ffmpegPath = binDir.appendingPathComponent("ffmpeg")

        // 두 번째 실행부터는 이미 설치되어 있으므로 setup 화면을 건너뛴다
        let alreadyInstalled = FileManager.default.fileExists(atPath: ytdlpPath.path)
        isYtdlpInstalled = alreadyInstalled
        isChecking = !alreadyInstalled

        checkRequirements()
    }

    func checkRequirements() {
        let alreadyInstalled = FileManager.default.fileExists(atPath: ytdlpPath.path)

        if !alreadyInstalled {
            DispatchQueue.main.async {
                self.isChecking = true
                self.setupStatus = "필요한 플러그인 설치 중..."
            }
        }

        DispatchQueue.global(qos: .userInitiated).async {
            try? FileManager.default.createDirectory(at: self.binDir, withIntermediateDirectories: true)

            self.ensureFfmpeg()
            let ffmpegVer = self.getFfmpegVersion()

            var installed = false
            var version = ""

            if FileManager.default.fileExists(atPath: self.ytdlpPath.path) {
                print("[EnvironmentChecker] yt-dlp_macos found at: \(self.ytdlpPath.path)")
                installed = true
                version = self.getLocalVersion()
            } else {
                DispatchQueue.main.async { self.setupStatus = "버전 확인 중..." }
                if self.copyBundledYtdlp() {
                    print("[EnvironmentChecker] copied from bundle to: \(self.ytdlpPath.path)")
                    installed = true
                    version = self.getLocalVersion()
                } else {
                    print("[EnvironmentChecker] yt-dlp_macos not found — binDir: \(self.binDir.path)")
                }
            }

            DispatchQueue.main.async {
                self.isYtdlpInstalled = installed
                self.ytdlpVersion = version
                self.ffmpegVersion = ffmpegVer
                self.isChecking = false
                print("[EnvironmentChecker] isYtdlpInstalled=\(installed), ytdlpVersion='\(version)', ffmpegVersion='\(ffmpegVer)'")

                if installed {
                    Task { await self.checkLatestVersionsAsync() }
                }
            }
        }
    }

    // ffmpeg가 binDir에 설치되어 있는지 확인 (없으면 번들에서 복사 시도)
    private func ensureFfmpeg() {
        if FileManager.default.fileExists(atPath: ffmpegPath.path) {
            print("[EnvironmentChecker] ffmpeg found at: \(ffmpegPath.path)")
            return
        }
        guard let bundlePath = Bundle.main.path(forResource: "ffmpeg", ofType: nil) else {
            print("[EnvironmentChecker] ffmpeg not found in bundle — 오디오 병합이 동작하지 않습니다")
            return
        }
        do {
            try FileManager.default.copyItem(atPath: bundlePath, toPath: ffmpegPath.path)
            try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: ffmpegPath.path)
            print("[EnvironmentChecker] ffmpeg copied from bundle to: \(ffmpegPath.path)")
        } catch {
            print("[EnvironmentChecker] ffmpeg copy error: \(error)")
        }
    }

    func checkLatestVersionsAsync() async {
        async let ytdlpTask: () = checkLatestYtdlpVersion()
        async let ffmpegTask: () = checkLatestFfmpegVersion()
        _ = await (ytdlpTask, ffmpegTask)
    }

    private func checkLatestYtdlpVersion() async {
        guard let url = URL(string: githubApiUrl) else { return }
        do {
            var request = URLRequest(url: url)
            request.setValue("NupyYTDownloader/1.0", forHTTPHeaderField: "User-Agent")
            request.timeoutInterval = 30
            let (data, _) = try await URLSession.shared.data(for: request)
            let release = try JSONDecoder().decode(GitHubRelease.self, from: data)
            let version = release.tagName.trimmingCharacters(in: CharacterSet(charactersIn: "v"))
            await MainActor.run { self.latestVersion = version }
        } catch {
            await MainActor.run { self.latestVersion = "" }
        }
    }

    private func checkLatestFfmpegVersion() async {
        guard let url = URL(string: ffmpegInfoUrl) else { return }
        do {
            var request = URLRequest(url: url)
            request.setValue("NupyYTDownloader/1.0", forHTTPHeaderField: "User-Agent")
            request.timeoutInterval = 30
            let (data, _) = try await URLSession.shared.data(for: request)
            let info = try JSONDecoder().decode(FfmpegInfo.self, from: data)
            await MainActor.run { self.latestFfmpegVersion = info.version }
            print("[EnvironmentChecker] latestFfmpegVersion='\(info.version)'")
        } catch {
            await MainActor.run { self.latestFfmpegVersion = "" }
            print("[EnvironmentChecker] checkLatestFfmpegVersion error: \(error)")
        }
    }

    // 버튼 한 번으로 yt-dlp / ffmpeg 둘 다 (필요한 것만) 업데이트
    func updatePlugins() async -> Bool {
        await MainActor.run {
            self.isUpdating = true
            self.updateProgress = 0
        }

        var success = true
        let ytdlpNeeded = ytdlpUpdateAvailable || !FileManager.default.fileExists(atPath: ytdlpPath.path)
        let ffmpegNeeded = !FileManager.default.fileExists(atPath: ffmpegPath.path)

        if ytdlpNeeded {
            success = await downloadAndInstallYtdlp() && success
        }
        if ffmpegNeeded {
            await MainActor.run { self.updateProgress = 0 }
            success = await downloadAndInstallFfmpeg() && success
        }

        await MainActor.run {
            self.updateProgress = 1.0
            self.isUpdating = false
        }
        return success
    }

    private func downloadAndInstallYtdlp() async -> Bool {
        let zipTemp = binDir.appendingPathComponent("yt-dlp_macos.zip.tmp")
        do {
            try await downloadFile(from: ytdlpZipUrl, to: zipTemp)

            let internalDir = binDir.appendingPathComponent("_internal")
            if FileManager.default.fileExists(atPath: internalDir.path) {
                try FileManager.default.removeItem(at: internalDir)
            }
            if FileManager.default.fileExists(atPath: ytdlpPath.path) {
                try FileManager.default.removeItem(at: ytdlpPath)
            }

            try extractZip(at: zipTemp, to: binDir)
            try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: ytdlpPath.path)
            try? FileManager.default.removeItem(at: zipTemp)

            let newVersion = getLocalVersion()
            await MainActor.run {
                self.ytdlpVersion = newVersion
                self.latestVersion = newVersion
                self.isYtdlpInstalled = true
            }
            return true
        } catch {
            try? FileManager.default.removeItem(at: zipTemp)
            print("[EnvironmentChecker] downloadAndInstallYtdlp error: \(error)")
            return false
        }
    }

    private func downloadAndInstallFfmpeg() async -> Bool {
        let zipTemp = binDir.appendingPathComponent("ffmpeg.zip.tmp")
        do {
            try await downloadFile(from: ffmpegZipUrl, to: zipTemp)

            if FileManager.default.fileExists(atPath: ffmpegPath.path) {
                try FileManager.default.removeItem(at: ffmpegPath)
            }

            try extractZip(at: zipTemp, to: binDir)
            try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: ffmpegPath.path)
            try? FileManager.default.removeItem(at: zipTemp)

            let newVersion = getFfmpegVersion()
            await MainActor.run {
                self.ffmpegVersion = newVersion
                self.latestFfmpegVersion = newVersion
            }
            return true
        } catch {
            try? FileManager.default.removeItem(at: zipTemp)
            print("[EnvironmentChecker] downloadAndInstallFfmpeg error: \(error)")
            return false
        }
    }

    // 공통 다운로드 (진행률 0.0 ~ 0.9 까지 채움 — 0.9 ~ 1.0은 압축 해제 마진)
    private func downloadFile(from urlString: String, to destination: URL) async throws {
        guard let url = URL(string: urlString) else {
            throw NSError(domain: "EnvironmentChecker", code: -1,
                          userInfo: [NSLocalizedDescriptionKey: "잘못된 URL"])
        }
        var request = URLRequest(url: url)
        request.setValue("NupyYTDownloader/1.0", forHTTPHeaderField: "User-Agent")

        let (asyncBytes, response) = try await URLSession.shared.bytes(for: request)
        let totalBytes = response.expectedContentLength

        FileManager.default.createFile(atPath: destination.path, contents: nil)
        guard let fileHandle = FileHandle(forWritingAtPath: destination.path) else {
            throw NSError(domain: "EnvironmentChecker", code: -2,
                          userInfo: [NSLocalizedDescriptionKey: "파일 생성 실패"])
        }
        defer { fileHandle.closeFile() }

        var receivedBytes: Int64 = 0
        var buffer = Data(capacity: 65536)

        for try await byte in asyncBytes {
            buffer.append(byte)
            receivedBytes += 1
            if buffer.count >= 65536 {
                fileHandle.write(buffer)
                buffer.removeAll(keepingCapacity: true)
                if totalBytes > 0 {
                    let progress = Double(receivedBytes) / Double(totalBytes) * 0.9
                    await MainActor.run { self.updateProgress = progress }
                }
            }
        }
        if !buffer.isEmpty { fileHandle.write(buffer) }
        await MainActor.run { self.updateProgress = 0.9 }
    }

    // 앱 번들의 yt-dlp_macos + _internal/ 를 binDir로 복사
    private func copyBundledYtdlp() -> Bool {
        guard let binaryPath = Bundle.main.path(forResource: "yt-dlp_macos", ofType: nil),
              let internalPath = Bundle.main.path(forResource: "_internal", ofType: nil) else {
            print("[EnvironmentChecker] bundle does not contain yt-dlp_macos or _internal")
            return false
        }
        do {
            if FileManager.default.fileExists(atPath: ytdlpPath.path) {
                try FileManager.default.removeItem(at: ytdlpPath)
            }
            try FileManager.default.copyItem(atPath: binaryPath, toPath: ytdlpPath.path)
            try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: ytdlpPath.path)

            let destInternal = binDir.appendingPathComponent("_internal")
            if FileManager.default.fileExists(atPath: destInternal.path) {
                try FileManager.default.removeItem(at: destInternal)
            }
            try FileManager.default.copyItem(atPath: internalPath, toPath: destInternal.path)
            return true
        } catch {
            print("[EnvironmentChecker] copyBundledYtdlp error: \(error)")
            return false
        }
    }

    private func extractZip(at zipPath: URL, to destDir: URL) throws {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/unzip")
        process.arguments = ["-o", zipPath.path, "-d", destDir.path]
        process.standardOutput = Pipe()
        process.standardError = Pipe()
        try process.run()
        process.waitUntilExit()
        guard process.terminationStatus == 0 else {
            throw NSError(domain: "EnvironmentChecker", code: Int(process.terminationStatus),
                          userInfo: [NSLocalizedDescriptionKey: "unzip 실패 (code \(process.terminationStatus))"])
        }
    }

    private func getLocalVersion() -> String {
        let process = Process()
        process.executableURL = ytdlpPath
        process.arguments = ["--version"]
        let stdoutPipe = Pipe()
        let stderrPipe = Pipe()
        process.standardOutput = stdoutPipe
        process.standardError = stderrPipe
        do {
            try process.run()
            let stdoutData = stdoutPipe.fileHandleForReading.readDataToEndOfFile()
            let stderrData = stderrPipe.fileHandleForReading.readDataToEndOfFile()
            process.waitUntilExit()

            let result = String(data: stdoutData, encoding: .utf8)?
                .trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
            let errOutput = String(data: stderrData, encoding: .utf8)?
                .trimmingCharacters(in: .whitespacesAndNewlines) ?? ""

            print("[EnvironmentChecker] getLocalVersion() exitCode=\(process.terminationStatus) stdout='\(result)' stderr='\(errOutput)'")
            return result
        } catch {
            print("[EnvironmentChecker] getLocalVersion() error: \(error)")
            return ""
        }
    }

    // ffmpeg -version 첫 줄에서 버전 토큰 파싱 (예: "ffmpeg version 8.0 ...")
    private func getFfmpegVersion() -> String {
        guard FileManager.default.fileExists(atPath: ffmpegPath.path) else { return "" }
        let process = Process()
        process.executableURL = ffmpegPath
        process.arguments = ["-version"]
        let stdoutPipe = Pipe()
        process.standardOutput = stdoutPipe
        process.standardError = Pipe()
        do {
            try process.run()
            let data = stdoutPipe.fileHandleForReading.readDataToEndOfFile()
            process.waitUntilExit()
            let output = String(data: data, encoding: .utf8) ?? ""
            let firstLine = output.components(separatedBy: "\n").first ?? ""
            let tokens = firstLine.components(separatedBy: " ")
            if tokens.count >= 3 {
                print("[EnvironmentChecker] getFfmpegVersion() → '\(tokens[2])'")
                return tokens[2]
            }
            return ""
        } catch {
            print("[EnvironmentChecker] getFfmpegVersion() error: \(error)")
            return ""
        }
    }

    private struct GitHubRelease: Decodable {
        let tagName: String
        enum CodingKeys: String, CodingKey { case tagName = "tag_name" }
    }

    private struct FfmpegInfo: Decodable {
        let version: String
    }
}
