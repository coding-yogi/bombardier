use log::error;
use std::collections::HashMap;

use crate::model::Request;

pub fn process(request: Request, env_map: &HashMap<String, String>) -> Request {
    let mut s_request = serde_json::to_string(&request).expect("Request cannot be serialized");
    s_request = param_substitution(s_request, &env_map);
    match serde_json::from_str(&s_request) {
        Ok(r) => r,
        Err(err) => {
            error!("Unable to deserialize request object after parameter replacement. Returning original request");
            error!("String: {}, Error: {}", s_request, err);
            request
        }
    }
}

pub fn param_substitution(mut content: String, params: &HashMap<String, String>) -> String {
    if content.contains("{{") { //Avoid unnecessary looping, might be tricked by json but would avoid most
        for (param_name, param_value) in params {
            let from = &format!("{{{{{}}}}}", param_name);
            let to = &param_value.replace(r#"""#, r#"\""#);
            content = content.replace(from, &to);
        }
    }
    
    content
}