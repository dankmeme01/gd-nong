use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{anyhow, Result};
use directories::UserDirs;
use native_dialog::FileDialog;
use tempfile::NamedTempFile;

fn main() {
    // temp files
    let mut to_remove_files: Vec<NamedTempFile> = vec![];

    let mut args = env::args();
    args.next();
    let song_id = args.next();
    if song_id.is_none() {
        print_help(false);
    }

    let song_id = song_id.unwrap();
    if song_id == "help" || song_id == "--help" || song_id == "-help" {
        print_help(true);
    }

    let song_id = match song_id.parse::<u32>() {
        Ok(x) => x,
        Err(e) => {
            eprintln!("Error parsing song ID: {e}");
            print_help(false);
        }
    };

    let gd_dir = match find_gd_dir() {
        Some(x) => x,
        None => {
            println!("Songs directory could not be found, please pick manually.");
            if let Some(result) = FileDialog::new().show_open_single_dir().unwrap() {
                result
            } else {
                eprintln!("could not locate the GD songs directory and user didn't provide one.");
                std::process::exit(1);
            }
        }
    };

    let song_arg = match args.next() {
        Some(x) => {
            if x.starts_with("http") {
                match download_file(&x) {
                    Err(x) => {
                        eprintln!("{x}");
                        std::process::exit(1);
                    }
                    Ok(x) => {
                        let path = x.path().to_path_buf();
                        to_remove_files.push(x);
                        path
                    }
                }
            } else {
                PathBuf::from(x)
            }
        }
        None => match get_song_file() {
            Some(x) => x,
            None => {
                eprintln!("no song provided.");
                std::process::exit(1);
            }
        },
    };

    match copy_maybe_convert(&song_arg, &gd_dir.join(format!("{song_id}.mp3"))) {
        Ok(_) => println!("Success"),
        Err(e) => println!("Error: {e}"),
    }

    to_remove_files.clear();
}

fn find_gd_dir() -> Option<PathBuf> {
    let os = std::env::consts::OS;

    let path = match os {
        "windows" => {
            let local_app_data = std::env::var("LOCALAPPDATA").ok()?;
            Path::new(&local_app_data).join("GeometryDash")
        }
        "linux" => {
            if let Some(user_dirs) = UserDirs::new() {
                let home_dir = user_dirs.home_dir();
                home_dir.join(".local/share/Steam/steamapps/compatdata/322170/pfx/drive_c/users/steamuser/AppData/Local/GeometryDash")
            } else {
                return None;
            }
        }
        "macos" => {
            if let Some(user_dirs) = UserDirs::new() {
                let home_dir = user_dirs.home_dir();
                home_dir.join("Library/Caches")
            } else {
                return None;
            }
        }
        _ => return None,
    };
    if path.exists()
        && path.is_dir()
        && path
            .read_dir()
            .map(|mut dir| dir.next().is_some())
            .unwrap_or(false)
    {
        Some(path)
    } else {
        None
    }
}

fn copy_maybe_convert(source: &Path, destination: &Path) -> Result<()> {
    let ext1 = source.extension().unwrap();
    let ext2 = destination.extension().unwrap();

    if ext1 == ext2 {
        std::fs::copy(source, destination)?;
        return Ok(());
    }

    let ffmpeg_dir = env::var("FFMPEG_PATH");
    let ffmpeg: PathBuf;

    if let Ok(ffmpeg_dir) = ffmpeg_dir {
        ffmpeg = Path::new(&ffmpeg_dir).join("ffmpeg");
    } else {
        ffmpeg = Path::new("ffmpeg").to_owned();
    }

    let output = Command::new(ffmpeg)
        .args(["-i", source.to_str().unwrap()])
        .arg("-vn")
        .args(["-ar", "44100"])
        .args(["-ac", "2"])
        .args(["-b:a", "192k"])
        .arg(destination)
        .output()?;

    if !output.status.success() {
        eprintln!("ffmpeg error!");
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        return Err(anyhow!("failed to run ffmpeg"));
    }

    Ok(())
}

fn download_file(url: &str) -> Result<NamedTempFile> {
    let extension = match url.split(".").last() {
        Some(x) => format!(".{x}"),
        None => "".to_string(),
    };
    let res = ureq::get(url).call()?;

    if res.status() == 200 {
        let mut dest = tempfile::Builder::new()
            .prefix("gd-nong-temp")
            .suffix(&extension)
            .tempfile()?;
        let mut content = res.into_reader();
        std::io::copy(&mut content, &mut dest)?;

        Ok(dest)
    } else {
        Err(anyhow!("URL request error: {}", res.status_text()))
    }
}

fn get_song_file() -> Option<PathBuf> {
    FileDialog::new()
        .add_filter(
            "Music Files",
            &["wav", "mp3", "ogg", "flac", "aac", "m4a", "wav", "opus"],
        )
        .show_open_single_file()
        .unwrap()
}

fn print_help(requested: bool) -> ! {
    let exe = env::current_exe().unwrap();
    let exe_fn = exe.file_name().unwrap().to_str().unwrap();
    println!("Usage: {exe_fn} <song ID> [path or URL]");
    std::process::exit(if requested { 0 } else { 1 });
}
