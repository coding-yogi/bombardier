use log::{error};
use rustc_hash::FxHashMap as HashMap;

use std::borrow::Cow;

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

pub fn param_substitution<'a>(content: &'a str, params: &HashMap<String, String>) -> Cow<'a,str> {
    if content.contains("{{") { //Avoid unnecessary looping, might be tricked by json but would avoid most
        return params.iter()
            .fold(Cow::from(content), |mut s, (f,t)| {
                let from = &["{{" , f , "}}"].join("");
                if content.contains(from) {
                    let to = &t.replace(r#"""#, r#"\""#);
                    s = s.replace(from,to).into();
                 }

                 s
            })
    } 

    content.into()
}