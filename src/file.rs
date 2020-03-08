use std::fs;
use std::collections::HashMap;
//use log::{info, debug};

pub fn get_content(path: &str) -> String {
    let content: String = fs::read_to_string(path)
        .expect("Something went wrong reading the file");

    content
}

pub fn find_and_replace(mut content: String, map: HashMap<String, String>) -> String {
    for k in map.keys() {
        let replaced_string = &format!("{{{{{}}}}}", k);
    
        let replacing_string = map.get(k).unwrap();
        //println!("replacing {} with {}", replaced_string, replacing_string);
        content = content.replace(replaced_string, replacing_string);
    }

    content
}