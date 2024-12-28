use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::io::{self, Write};
use walkdir::WalkDir;
use std::env; // std::env로 수정
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

    // 파일/폴더 개수와 함께 청크 개수 출력
    let chunk_size = 100; // 한 청크 당 처리할 파일 수
    let chunk_count = (files_and_folders.len() + chunk_size - 1) / chunk_size; // 총 청크 수 계산
    println!("\n검색된 총 파일 및 폴더 수: {}", files_and_folders.len());
    println!("이 파일/폴더들을 {}개의 청크로 나누어 처리합니다.\n", chunk_count);
    println!("삭제를 진행하시겠습니까? (y/n): ");
    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("입력 오류");

    if input.trim().to_lowercase() == "y" {
        // 청크 단위로 삭제 처리
        let failed_deletions = delete_files_in_chunks(&files_and_folders, chunk_size);

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
    }
}

// `walkdir`을 사용하여 파일과 폴더를 검색
fn collect_files_and_folders(path: &Path) -> Vec<PathBuf> {
    let mut files_and_folders = Vec::new();
    let total_files = WalkDir::new(path)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .count();

    println!("검색할 총 파일/폴더 개수: {}\n", total_files);

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
    if total == 0 {
        println!("진행 상태를 계산할 수 없습니다: 총 파일 수가 0입니다.");
        return;
    }

    let progress = (current as f32) / (total as f32); // 진행 비율
    let total_blocks = 50; // 막대의 총 블록 수
    let ratio = progress * (total_blocks as f32); // 현재 진행률 계산
    let filled = ratio.floor() as usize; // 채워진 블록 수
    let remainder_index = ((ratio - (filled as f32)) * 8.0 + 0.5).floor() as usize; // 남은 부분의 세부 블록
    let bar = ["　", "▏", "▎", "▍", "▌", "▋", "▊", "▉", "█"]; // 그래프 요소

    let mut bar_display = String::new();

    // 막대 생성
    for _ in 0..filled {
        bar_display.push_str(bar[8]); // 완전히 채워진 블록
    }
    if filled < total_blocks {
        bar_display.push_str(bar[remainder_index]); // 남은 부분
    }
    for _ in filled + 1..total_blocks {
        bar_display.push_str(bar[0]); // 공백
    }

    // 덮어쓰기 보장
    let clear_line = format!("\r{: <width$}", "", width = 60); // 너비 60으로 초기화
    print!("{}", clear_line); // 기존 출력 삭제

    // 퍼센트와 진행 바를 같은 줄에 출력
    print!("\r{}% {}", (progress * 100.0).round(), bar_display);

    io::stdout().flush().unwrap(); // 즉시 출력
}

// 삭제 작업을 청크 단위로 수행하는 함수, 실패한 파일/폴더는 반환
fn delete_files_in_chunks(files_and_folders: &[PathBuf], chunk_size: usize) -> Vec<PathBuf> {
    println!("\n삭제를 시작합니다...");
    let start_time = Instant::now();
    let mut failed_deletions = Vec::new();
    let total_files = files_and_folders.len();

    // 파일을 청크 단위로 나누어서 삭제
    let chunks = files_and_folders.chunks(chunk_size);

    for (chunk_index, chunk) in chunks.enumerate() {
        let chunk_start_time = Instant::now();

        for (index, path) in chunk.iter().enumerate() {
            if path.is_file() {
                if let Err(e) = fs::remove_file(path) {
                    eprintln!("파일 삭제 실패: {} - {}", path.display(), e);
                    failed_deletions.push(path.to_path_buf()); // 실패한 파일 저장
                } else {
                    print!("\r파일 삭제 완료: {}", path.display()); // 줄 바꿈 없이 출력
                    io::stdout().flush().unwrap(); // 즉시 출력
                }
            } else if path.is_dir() {
                if let Err(e) = fs::remove_dir_all(path) {
                    eprintln!("폴더 삭제 실패: {} - {}", path.display(), e);
                    failed_deletions.push(path.to_path_buf()); // 실패한 폴더 저장
                } else {
                    print!("\r폴더 삭제 완료: {}", path.display()); // 줄 바꿈 없이 출력
                    io::stdout().flush().unwrap(); // 즉시 출력
                }
            }

            // 청크 내에서 진행 상황 출력
            print_progress((chunk_index * chunk_size) + index + 1, total_files);
        }

        let chunk_duration = chunk_start_time.elapsed();
        // 청크 완료 출력 (진행 상태 갱신)
        print!("\r{} 번째 청크 삭제 완료! (소요 시간: {:.2?})", chunk_index + 1, chunk_duration);
        io::stdout().flush().unwrap(); // 즉시 출력
    }

    let duration = start_time.elapsed();
    println!("\n전체 삭제 완료! (소요 시간: {:.2?})", duration);

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

// 실패한 파일/폴더를 실제로 삭제하는 함수
fn delete_files_and_folders(failed_deletions: &[PathBuf]) {
    for path in failed_deletions {
        if path.is_file() {
            if let Err(e) = fs::remove_file(path) {
                eprintln!("파일 삭제 실패: {} - {}", path.display(), e);
            } else {
                println!("파일 삭제 완료: {}", path.display());
            }
        } else if path.is_dir() {
            if let Err(e) = fs::remove_dir_all(path) {
                eprintln!("폴더 삭제 실패: {} - {}", path.display(), e);
            } else {
                println!("폴더 삭제 완료: {}", path.display());
            }
        }
    }
}
