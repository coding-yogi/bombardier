# bombardier
Rust based HTTP load testing tool using postman collection


Bombardier can take your existing postman collection and bombard your server with those requests under specified load

## Building from source
Make sure you have cargo and rust installed. Checkout the code and run below command   
`cargo build -- release`
  
  
## Config json
You need to create a json file which can tell Bombardier about the load configuration.  
If you do not wish to push stats to influxdb for real time monitoring you can skip that configuration. Stats would still be written to report file

```
{
    "environment_file": "./examples/environment.json",
    "collection_file": "./examples/collection.json",
    "thread_count": 1,
    "iterations": 1,
    "thread_delay": 1,
    "execution_time": 1,
    "rampup_time": 1,
    "report_file": "report.csv",
    "continue_on_error": false,
    "handle_cookies": false,
    "influxdb" : {
        "url": "http://localhost:8086",
        "username": "",
        "password": "",
        "dbname": "mydb"
    }
}
```

## Running Tests
`./bombardier bombard --config <path of config json>`

## Enabling debug logs
`export RUST_LOG=debug`

## Generating reports
`./bombardier report --config <path of config json>`

## Limitations
* Bombardier currently will on parse requests directly under collection folder and one sub folder. It will ignore requests from folders which or 2 or more levels down in hierarchy
* Bombardier currently cannot generate different loads for different folders under collection. Whole collection will be executed with same thread count
* Bombardier cannot parse or execute Postman's javascript written under `test` tag. Due to this limitation you should explicitly tell bombardier if you wish to extract any value from response to be used in following requests. Refer [postprocessor](https://github.com/coding-yogi/bombardier/blob/develop/docs/postprocessor.md) guide for the same
