//! 실제 네트워크로 설치 → 정보 조회 → 다운로드를 확인한다.
//!
//! cargo run -p nupy-core --example smoke -- <bin_dir> <out_dir> <url> [format_id]
use nupy_core::{ytdlp, DownloadRequest, FormatKind, ToolManager, ToolPaths};
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> nupy_core::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let paths = ToolPaths::new(&args[1]);
    let out_dir = std::path::PathBuf::from(&args[2]);
    let url = &args[3];
    let tm = ToolManager::new(paths.clone())?;
    let on = |p: nupy_core::InstallProgress| {
        if p.stage != nupy_core::InstallStage::Downloading || p.fraction.is_some_and(|f| ((f * 100.0) as u32).is_multiple_of(25)) {
            println!("  {:?} {:?} {:?}", p.tool, p.stage, p.fraction.map(|f| (f * 100.0).round()));
        }
    };

    let latest = tm.latest_ytdlp_version().await.ok();
    println!("latest = {latest:?}, local = {:?}", tm.local_ytdlp_version().await);
    if !paths.ytdlp_installed() {
        println!("installed yt-dlp {}", tm.install_ytdlp(latest.as_deref(), &on).await?);
    }
    if !paths.ffmpeg_installed() {
        tm.install_ffmpeg(&on).await?;
        println!("installed ffmpeg");
    }
    println!("status = {:?}", tm.status(true).await);

    let t = std::time::Instant::now();
    let info = ytdlp::fetch_info(&paths, url).await?;
    println!("title = {} ({:.1}s)", info.title, t.elapsed().as_secs_f32());
    for f in &info.formats {
        println!("  {:>6} {:?} {} {} {} {:?}", f.id, f.kind, f.label, f.codec_label, f.output_ext, f.filesize);
    }
    let fmt = match args.get(4) {
        Some(id) => info.formats.iter().find(|f| &f.id == id).unwrap(),
        None => info.formats.iter().rfind(|f| f.kind == FormatKind::VideoOnly).unwrap(),
    };
    let req = DownloadRequest {
        url: url.clone(),
        format_id: fmt.id.clone(),
        kind: fmt.kind,
        output: out_dir.join(format!("{} 100%.whatever", info.title.replace('/', "-"))),
    };
    let mut last = -1i32;
    let path = ytdlp::download(&paths, &req, CancellationToken::new(), |ev| match &ev {
        nupy_core::DownloadEvent::Progress { overall, .. } => {
            let p = (overall * 10.0) as i32;
            if p != last { last = p; println!("  {ev:?}"); }
        }
        _ => println!("  {ev:?}"),
    })
    .await?;
    println!("saved = {}", path.display());
    Ok(())
}
