use tokio:: {
    fs,
    io::Error
};

use std::collections::HashMap;

pub async fn get_content(path: &str) -> Result<String, Error> {
    Ok(fs::read_to_string(path).await?)
}

pub async fn create_file(path: &str) -> Result<fs::File, Error> {
    fs::File::create(path).await
}

pub async fn read_file(path: &str) -> Result<Vec<u8>, Error> {
    fs::read(path).await
}

pub async fn get_file(path: &str) -> Result<fs::File, Error> {
    fs::File::open(path).await
}

pub fn get_file_name(path: &str) -> Result<String, Error> {
    let iter = path.split("/");
    Ok(iter.last().unwrap().to_string())
}

pub fn param_substitution(mut content: String, params: &HashMap<String, String>) -> String {
    if content.contains("{{") { //Avoid unnecessary looping, might be tricked by json but would avoid most
        for (param_name, param_value) in params {
            let from = &format!("{{{{{}}}}}", param_name);
            let to = param_value.replace(r#"""#, r#"\""#);
            content = content.replace(from, &to);
        }
    }
    
    content
}