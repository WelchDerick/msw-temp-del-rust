fn main() {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("1.ico"); // 아이콘 파일 경로
        res.compile().expect("Failed to set icon");
    }
}