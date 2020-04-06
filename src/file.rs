use std::fs;
use std::process;
use std::collections::HashMap;

use log::{error};

pub fn get_content(path: &str) -> String {
    let content: String = match fs::read_to_string(path) {
        Err(err) => {
            error!("Something went wrong reading the file on path {} : {}", path, err);
            process::exit(-1)
        },
        Ok(s) => s
    };

    content
}

pub fn create_file(path: &str) -> fs::File {
    let report_file = fs::File::create(path); 
    match report_file {
        Ok(f) => f,
        Err(s) => {
            error!("Unable to create report file: {}", s);
            process::exit(-1);
        } 
    }
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