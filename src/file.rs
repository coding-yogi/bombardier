use std::fs;
use std::io::Error;
use std::collections::HashMap;

use log::error;

pub fn get_content(path: &str) -> Result<String, Error> {
    let file = fs::read_to_string(path);
    if file.is_err() {
        error!("Could not read file from path {}", path);
    }

    file
}

pub fn create_file(path: &str) -> Result<fs::File, Error> {
    let file = fs::File::create(path);
    if file.is_err() {
        error!("Could not create file on path {}", path);
    }

    file
}

pub fn read_file(path: &str) -> Result<Vec<u8>, Error> {
    let file = fs::read(path);
    if file.is_err() {
        error!("Could not read file on path {}", path);
    }

    file
}

pub fn get_file(path: &str) -> Result<fs::File, Error> {
    let file = fs::File::open(path);
    if file.is_err() {
        error!("Could not open file on path {}", path);
    }

    file
}

pub fn find_and_replace(mut content: String, map: &HashMap<String, String>) -> String {
    if content.contains("{{") { //Avoid unnecessary looping, might be tricked by json but would avoid most
        for (k, v) in map {
            let replaced_string = &format!("{{{{{}}}}}", k);
            let replacing_string = v.replace(r#"""#, r#"\""#);
            content = content.replace(replaced_string, &replacing_string);
        }
    }
    
    content
}