use crate::config;

use std::fs;

pub fn get_file_list(storage: &str) -> String {
    let mut msg = String::new();
    for entry in fs::read_dir(storage).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if !path.is_dir() {
            let fullpath = String::from(entry.path().to_string_lossy());
            let filename = String::from(str::replace(&fullpath, storage, ""));
            let file_name = &filename[1..];

            let file = fs::File::open(fullpath).unwrap();
            let file_size = file.metadata().unwrap().len();
            let file_info = format!("{}  [{} bytes]", file_name, file_size);
            msg.push_str(&file_info);
            msg.push('\n');
        }
    }
    msg
}

pub fn check_file(file_name: &str, storage: &str) -> config::ServerFile {
    let mut f = config::ServerFile::new();

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
