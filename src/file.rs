use std::fs;
use std::io::Error;
use std::collections::HashMap;

pub fn get_content(path: &str) -> Result<String, Error> {
    fs::read_to_string(path)
}

pub fn create_file(path: &str) -> Result<fs::File, Error> {
    fs::File::create(path)
}

pub fn get_file(path: &str) -> Result<fs::File, Error> {
    fs::File::open(path)
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