use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::io::{self, Write};
use walkdir::WalkDir;
use std::env;
use std::process;

// 대상 폴더 경로를 가져오는 함수
fn get_target_folder() -> PathBuf {
    let temp_dir = env::var_os("TEMP")
        .and_then(|temp| {
            let path = PathBuf::from(temp).join("nexon/MapleStory Worlds");
            if path.exists() {
                Some(path)
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            // 홈 디렉토리가 없는 경우 에러 로그 출력 후 프로그램 종료
            let home_dir = dirs::home_dir().expect("Could not determine the user home directory");
            home_dir.join("AppData/Local/Temp/nexon/MapleStory Worlds")
        });

    temp_dir
}

fn main() {
    // 폴더 경로 가져오기
    let folder_path = get_target_folder();

    // 경로 존재 여부 확인
    if !folder_path.exists() {
        eprintln!("Error: 경로를 찾을 수 없습니다. 경로: {}", folder_path.display());
        process::exit(1); // 경로가 없으면 종료
    }

    println!("대상 폴더 경로: {}", folder_path.display());
    println!("\n파일 및 폴더 검색 중...\n");

    let start_time = Instant::now();

    // 파일 및 폴더 검색
    let files_and_folders = collect_files_and_folders(&folder_path);

    let duration = start_time.elapsed();
    println!("\n\n검색 완료! (소요 시간: {:.2?})", duration);

    if files_and_folders.is_empty() {
        eprintln!("폴더나 파일이 존재하지 않습니다.");
        return;
    }

    // 검색된 파일/폴더 출력
    println!("\n검색된 총 파일 및 폴더 수: {}", files_and_folders.len());
    println!("삭제를 진행하시겠습니까? (y/n): ");
    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("입력 오류");

    if input.trim().to_lowercase() == "y" {
        let failed_deletions = delete_files_and_folders(&files_and_folders);

        // 삭제 실패한 항목들 다시 시도
        if !failed_deletions.is_empty() {
            println!("\n삭제 실패한 항목들이 있습니다. 다시 시도하시겠습니까? (y/n): ");
            let mut retry_input = String::new();
            io::stdin().read_line(&mut retry_input).expect("입력 오류");

            if retry_input.trim().to_lowercase() == "y" {
                delete_files_and_folders(&failed_deletions);
            } else {
                println!("삭제 작업을 취소했습니다.");
            }

            // 오류 메시지를 파일로 저장
            if let Err(e) = save_errors_to_file(&failed_deletions) {
                eprintln!("오류 파일 저장 실패: {}", e);
            } else {
                println!("삭제 실패 오류가 error_log.txt 파일에 저장되었습니다.");
            }
        }
    } else {
        println!("삭제 작업을 취소했습니다.");
    }
}

// `walkdir`을 사용하여 파일과 폴더를 검색
fn collect_files_and_folders(path: &Path) -> Vec<PathBuf> {
    let mut files_and_folders = Vec::new();
    let total_files = WalkDir::new(path)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .count();

    println!("검색할 총 파일/폴더 개수: {}", total_files);

    let mut current = 0;
    WalkDir::new(path)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .for_each(|entry| {
            files_and_folders.push(entry.path().to_path_buf());
            current += 1;
            print_progress(current, total_files);
        });

    files_and_folders
}

fn print_progress(current: usize, total: usize) {
    let progress = (current as f32) / (total as f32); // 진행 비율
    let bar_width = 50; // 로딩 바의 너비

    // 채워진 부분
    let filled_width = (progress * bar_width as f32).round() as usize;

    // 나머지 부분
    let empty_width = bar_width - filled_width;

    // 채워진 부분은 '█', 비어있는 부분은 ' '로 채움
    let filled_bar = "█".repeat(filled_width);
    let empty_bar = " ".repeat(empty_width);

    // 진행 상태를 출력
    print!("\r진행 중: [{}{}] {:.2}%", filled_bar, empty_bar, progress * 100.0);
    io::stdout().flush().unwrap();
}

// 삭제 작업을 수행하는 함수, 실패한 파일/폴더는 반환
fn delete_files_and_folders(files_and_folders: &[PathBuf]) -> Vec<PathBuf> {
    println!("\n삭제를 시작합니다...");
    let start_time = Instant::now();
    let mut failed_deletions = Vec::new();
    let total_files = files_and_folders.len();

    files_and_folders
        .iter()
        .enumerate()
        .for_each(|(index, path)| {
            if path.is_file() {
                if let Err(e) = fs::remove_file(path) {
                    eprintln!("파일 삭제 실패: {} - {}", path.display(), e);
                    failed_deletions.push(path.to_path_buf()); // 실패한 파일 저장
                } else {
                    println!("파일 삭제 완료: {}", path.display());
                }
            } else if path.is_dir() {
                if let Err(e) = fs::remove_dir_all(path) {
                    eprintln!("폴더 삭제 실패: {} - {}", path.display(), e);
                    failed_deletions.push(path.to_path_buf()); // 실패한 폴더 저장
                } else {
                    println!("폴더 삭제 완료: {}", path.display());
                }
            }

            // 삭제 진행률 출력
            print_progress(index + 1, total_files);
        });

    let duration = start_time.elapsed();
    println!("\n삭제 완료! (소요 시간: {:.2?})", duration);

    failed_deletions // 실패한 항목을 반환
}

// 실패한 항목들을 텍스트 파일에 저장하는 함수
fn save_errors_to_file(failed_deletions: &[PathBuf]) -> io::Result<()> {
    let mut file = File::create("error_log.txt")?; // 파일 열기, 없으면 생성
    for path in failed_deletions {
        writeln!(file, "삭제 실패: {}", path.display())?; // 실패한 항목 기록
    }
    Ok(())
}
