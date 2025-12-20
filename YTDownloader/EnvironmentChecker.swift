import Foundation

// 2. 시스템 요구사항 체크 로직
class EnvironmentChecker: ObservableObject {
    @Published var isYtdlpInstalled = false
    @Published var isFfmpegInstalled = false
    @Published var isChecking = true
    
    init() { checkRequirements() }
    
    func checkRequirements() {
        self.isChecking = true
        // 실제 환경에서는 'which' 명령어로 체크
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.0) {
            self.isYtdlpInstalled = true // 테스트를 위해 true 설정
            self.isFfmpegInstalled = true
            self.isChecking = false
        }
    }
}
