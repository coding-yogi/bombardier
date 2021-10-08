use log::{error, warn};
use lazy_static::lazy_static;
use regex::Regex;
use rustc_hash::FxHashMap as HashMap;

use crate::model::Request;

pub fn process(request: &Request, env_map: &HashMap<String, String>) -> Request {
    if !env_map.is_empty() {
        let s_request = serde_json::to_string(&request).expect("Request cannot be serialized");
        let s_request = param_substitution(&s_request, env_map);

        if let Ok(new_request) = serde_json::from_str::<Request>(&s_request) {
            return new_request;
        } else {
            error!("Unable to deserialize request object after parameter replacement. Returning original request");
        }
    }

    request.to_owned()
}

pub fn param_substitution(content: &str, params: &HashMap<String, String>) -> String {
   let mut new_value = String::from(content);

    lazy_static! {
        static ref RE: Regex = Regex::new(r"\{\{(\w+)\}\}").unwrap();
    }

    if content.contains("{{") {
        for cap in RE.captures_iter(content) {
            if let Some(map_value) = params.get(&cap[1]) {
                new_value = new_value.replace(&cap[0], map_value);
            } else {
                warn!("Param {} not found in env map", &cap[1]);
            }
        }
    }

    new_value
}