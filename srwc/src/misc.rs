use crate::config::ServerFile;

use console::{Term, style};
use regex::Regex;
use std::fs;

pub fn srwc_prompt() {
    let term = Term::stdout();
    term.write_str("SRWC> ").unwrap();
}

pub fn srwc_help(){
    println!("{}", style("Available SRWC commands:").magenta());
    println!("{} {}\t-> {}", style("get").green(), style("\"filename\"").blue(), style("Download file from server").cyan());
    println!("{} {}\t-> {}", style("put").green(), style("\"filename\"").blue(), style("Upload file to server").cyan());
    println!("{} {}\t-> {}", style("rm").green(), style("\"filename\"").blue(), style("Remove file in server").cyan());
    println!("{}\t\t-> {}", style("ls").green(), style("Show files in server").cyan());
    println!("{}\t\t-> {}", style("help").green(), style("Show available commands").cyan());
    println!("{}\t\t-> {}", style("quit").green(), style("Quit SRWC").cyan());
}

pub fn file_list_response(response: &String) -> Vec<ServerFile> {
    let mut file_list: Vec<ServerFile> = Vec::new();
    let file_name_regex: Regex = Regex::new(r"(.*)(?:\s\s\[.*\sbytes\][\n\r])").unwrap();
    let file_size_regex: Regex = Regex::new(r"(\d+)(?:\sbytes\][\n\r])").unwrap();

    for cp in file_name_regex.captures_iter(response).enumerate() {
        let mut f = ServerFile::new();
        f.name = String::from(&cp.1[1]);
        file_list.push(f);
    }

    for (i, cap) in file_size_regex.captures_iter(response).enumerate() {
        file_list[i].size = cap[1].parse::<u64>().unwrap();
    }

    file_list
}

pub fn check_file(file_name: &str, storage: &str) -> ServerFile {
    let mut f = ServerFile::new();

    for entry in fs::read_dir(storage).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if !path.is_dir() {
            let fullpath = String::from(entry.path().to_string_lossy());
            let filename = String::from(str::replace(&fullpath, storage, ""));

            if &filename[1..] == file_name {
                let file = fs::File::open(&fullpath).unwrap();
                f.size = file.metadata().unwrap().len();
                f.name = file_name.to_string();
                f.fullpath = fullpath;
                break;
            }
        }
    }

    f
}
