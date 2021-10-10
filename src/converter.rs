
use reqwest::Request as Reqwest;
use reqwest::{
    RequestBuilder,
    header::{
        HeaderMap, 
        HeaderName, 
        HeaderValue
    },
    Method,
    multipart::{Form,Part}
};
use rustc_hash::FxHashMap as HashMap;
use tokio::fs;

use std::{
    str::FromStr,
    error::Error as StdError
};

use crate::model::{Request, Body};
use crate::protocol::http::HttpClient;

pub async fn convert_request(http_client: &HttpClient, request: &Request) -> Result<Reqwest, Box<dyn StdError + Send + Sync>> {
    //Method
    let method = Method::from_str(&request.method)?;

    //Headers
    let headers = match get_header_map_from_request(request).await {
        Ok(headers) => headers,
        Err(err) => return Err(err)
    };

     //Create builder
     let mut builder = http_client
        .get_client()
        .request(method, request.url.to_owned()).headers(headers);

    //Body
    let body = &request.body;

    if !body.raw.is_empty()  {
        builder = builder.body(body.raw.to_owned());
    } else if !body.formdata.is_empty() {
        builder = add_multipart_form_data(builder, body).await?;
    } else if !body.urlencoded.is_empty() {
        builder = add_url_encoded_data(builder, body).await;
    } 

    Ok(builder.build()?)
}

async fn get_header_map_from_request(request: &Request) 
-> Result<HeaderMap, Box<dyn std::error::Error + Send + Sync>> {
    let mut headers = HeaderMap::new();
    for header in &request.headers {
        headers.insert(HeaderName::from_str(header.0.as_str().unwrap())?, 
        HeaderValue::from_str(header.1.as_str().unwrap())?);
    }

    Ok(headers)
}

async fn add_multipart_form_data(builder: RequestBuilder, body: &Body) 
-> Result<RequestBuilder, Box<dyn std::error::Error + Send + Sync>> {
    let mut form = Form::new();

    for data in &body.formdata {
        let param_key = data.0.as_str().unwrap();
        let param_value = data.1.as_str().unwrap();

        if param_key.to_lowercase().starts_with("_file") {
            let contents = fs::read_to_string(param_value).await?;
            let file_name = get_file_name(param_value)?;
            let part = Part::stream(contents).file_name(file_name)
                                .mime_str("application/octet-stream").unwrap();
            form = form.part("", part);
        } else {
            form = form.text(param_key.to_owned(), param_value.to_owned());
        }
    }

    Ok(builder.multipart(form))
}

fn get_file_name(path: &str) -> Result<String, tokio::io::Error> {
    let iter = path.split('/');
    Ok(iter.last().unwrap().to_string())
}

async fn add_url_encoded_data(builder: RequestBuilder, body: &Body) -> RequestBuilder {
    let mut params = HashMap::default();

    body.urlencoded.iter().for_each(|(k,v)| {
        params.insert(k.as_str().unwrap().to_owned(), v.as_str().unwrap().to_owned());
    });

    builder.form(&params)
}

#[cfg(test)]
mod tests {
    use crate::converter::{
        convert_request,
        get_file_name, 
        get_header_map_from_request
    };
    use crate::model::Request;
    use crate::protocol::http::HttpClient;

    use std::str::from_utf8;

    #[test]
    fn test_get_file_name_using_abs_path() {
        let file_path = "/home/bombardier/names.txt";
        let file_name = get_file_name(file_path).unwrap();
        assert_eq!(file_name, "names.txt");
    }

    #[test]
    fn test_get_file_name_using_rel_path() {
        let file_path = "./names.txt";
        let file_name = get_file_name(file_path).unwrap();
        assert_eq!(file_name, "names.txt");
    }

    #[test]
    fn test_get_file_name_using_empty_path() {
        let file_path = "";
        let file_name = get_file_name(file_path).unwrap();
        assert_eq!(file_name, "");
    }

    #[tokio::test]
    async fn test_get_header_map_with_no_headers() {
        let request_yaml = r"
        name: echoGet
        method: GET
        url: 'https://google.com/'";
        
        let request = serde_yaml::from_str::<Request>(request_yaml).unwrap();
        let headers = get_header_map_from_request(&request).await.unwrap();
        assert!(headers.is_empty())
    }

    #[tokio::test]
    async fn test_get_header_map_with_multiple_headers() {
        let request_yaml = r"
        name: echoGet
        method: GET
        url: 'https://google.com/'
        headers:
          authorization: 'jwt some_token_value'
          accept: 'application/json'";
        
        let request = serde_yaml::from_str::<Request>(request_yaml).unwrap();
        let headers = get_header_map_from_request(&request).await.unwrap();
        assert!(headers.len() == 2);
        assert!(headers.get("accept").unwrap().to_str().unwrap() == "application/json")
    }

    #[tokio::test]
    async fn test_convert_get_request() {
        let request_yaml = r"
        name: echoGet
        method: GET
        url: 'https://google.com/'
        headers:
          authorization: 'jwt some_token_value'
          accept: 'application/json'";

        let request = serde_yaml::from_str::<Request>(request_yaml).unwrap();
        let client = HttpClient::get_default_async_client().unwrap();
        
        let reqwest = convert_request(&client, &request).await;
        assert!(reqwest.is_ok());

        let reqwest = reqwest.unwrap();
        assert!(reqwest.method().as_str() == "GET");
        assert!(reqwest.url().as_str() == "https://google.com/");
        assert!(reqwest.headers().len() == 2);
    }

    #[tokio::test]
    async fn test_convert_urlencoded_request() {
        let request_yaml = r"
        name: echoGet
        method: POST
        url: 'https://google.com/'
        body:
          urlencoded:
            key1: value1
            key2: value2";

        let request = serde_yaml::from_str::<Request>(request_yaml).unwrap();
        let client = HttpClient::get_default_async_client().unwrap();
        
        let reqwest = convert_request(&client, &request).await;
        assert!(reqwest.is_ok());

        let reqwest = reqwest.unwrap();
        assert!(reqwest.method().as_str() == "POST");
        assert!(reqwest.url().as_str() == "https://google.com/");
        assert!(reqwest.body().is_some());

        let str_body = from_utf8(reqwest.body().unwrap().as_bytes().unwrap()).unwrap();
        assert_eq!(str_body,"key2=value2&key1=value1");
    }

    #[tokio::test]
    async fn test_convert_raw_request() {
        let request_yaml = r#"
        name: echoGet
        method: POST
        url: 'https://google.com/'
        body: 
          raw: '{ "test":"test" }'"#;

        let request = serde_yaml::from_str::<Request>(request_yaml).unwrap();
        let client = HttpClient::get_default_async_client().unwrap();
        
        let reqwest = convert_request(&client, &request).await.unwrap();
        assert!(reqwest.body().is_some());

        let str_body = from_utf8(reqwest.body().unwrap().as_bytes().unwrap()).unwrap();
        assert_eq!(str_body,r#"{ "test":"test" }"#);
    }
}