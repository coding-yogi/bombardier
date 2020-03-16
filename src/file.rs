use std::fs;
use std::collections::HashMap;
use log::{info};

pub fn get_content(path: &str) -> String {
    let content: String = fs::read_to_string(path)
        .expect("Something went wrong reading the file");

    content
}

pub fn find_and_replace(mut content: String, map: &HashMap<String, String>) -> String {
    info!("Replacing parameter values");
    for k in map.keys() {
        let replaced_string = &format!("{{{{{}}}}}", k);
        let replacing_string = map.get(k).unwrap();
        content = content.replace(replaced_string, replacing_string);
    }

    content
}