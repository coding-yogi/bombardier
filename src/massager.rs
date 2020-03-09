use crate::parser;

use log::{debug};
use base64::{encode_config, STANDARD};

pub fn massage(scenarios: &mut Vec<parser::Scenario>) {
    for scenario in scenarios {
        inject_basic_auth(scenario);
        //debug!("{:?}", scenario);
        
        for request in &mut scenario.requests {
            inject_basic_auth(request);
            //debug!("{:?}", request);
        }
    } 
}

fn inject_basic_auth<T: parser::HasRequestDetails>(item: &mut T) {
    let auth = &item.get_request_details().auth;
    if auth.auth_type == "basic" {
        
        let username = auth.basic
            .iter()
            .find(|kv| kv.key == "username")
            .unwrap()
            .value.clone();
        
        let password = auth.basic
            .iter()
            .find(|kv| kv.key == "password")
            .unwrap()
            .value.clone();

        let basic_auth =  encode_config(format!("{}:{}", &username, &password), STANDARD);
        item.get_request_details().headers.push(parser::KeyValue {
            key: String::from("authorization"),
            value: basic_auth,
        })
    }
}