# Bombardier
Rust based HTTP load testing tool using postman collection  
Bombardier can take your existing postman collection and bombard your server with those requests under specified load  
  
## Building from source
Make sure you have cargo and rust installed. Checkout the code and run below command   
`cargo build --release`  
  
## Config json
You need to create a json file which can tell Bombardier about the load configuration.  
If you do not wish to push stats to influxdb for real time monitoring you can skip that configuration. Stats would still be written to report file

```
{
    "environment_file": "./examples/environment.json",
    "collection_file": "./examples/collection.json",
    "data_file": "./examples/data.csv",
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

For more details regarding configuration json, please check [configurations](https://github.com/coding-yogi/bombardier/blob/develop/docs/configurations.md) doc.  

## Running Tests
`./bombardier bombard --config <path of config json>`

## Enabling debug mode for more logs
`export RUST_LOG=debug`

## Generating reports
`./bombardier report --config <path of config json>`  
  
## Sample report
| Request                         | Total Hits | Hits/s    | Min | Avg | Max  | 90% | 95% | 99% | Errors | Error Rate |
|---------------------------------|------------|-----------|-----|-----|------|-----|-----|-----|--------|------------|
| PostWithFormData                | 1000       | 5.7471266 | 235 | 282 | 1312 | 300 | 304 | 398 | 0      | 0          |
| PostWithJsonAndReplaceableParam | 1000       | 5.7471266 | 235 | 280 | 1308 | 296 | 304 | 335 | 0      | 0          |
| PostWithFormUrlEncoded          | 1000       | 5.7471266 | 235 | 281 | 882  | 296 | 304 | 783 | 0      | 0          |
| GetWithQueryParams              | 1000       | 5.7471266 | 234 | 284 | 1307 | 296 | 303 | 808 | 0      | 0          |
| PostWithNoBody                  | 1000       | 5.7471266 | 234 | 279 | 2168 | 296 | 303 | 327 | 0      | 0          |
  
  
| Total Execution Time (in secs) | Total Hits | Hits/s    | Total Errors | Error Rate |
|--------------------------------|------------|-----------|--------------|------------|
| 174                            | 6000       | 34.482758 | 0            | 0          |
  

## Limitations
* Bombardier currently will only parse the requests which are directly under collection folder or it's sub-folder. It will ignore requests from folders which are 2 or more levels down in hierarchy.
  In below example bombardier will ignore all requests under folder 2

collection  
&nbsp; &nbsp; &nbsp;|__ request1  
&nbsp; &nbsp; &nbsp;|__ folder1  
&nbsp; &nbsp; &nbsp; &nbsp; &nbsp; &nbsp; &nbsp;|__ request2  
&nbsp; &nbsp; &nbsp; &nbsp; &nbsp; &nbsp; &nbsp;|__ request3  
&nbsp; &nbsp; &nbsp; &nbsp; &nbsp; &nbsp; &nbsp;|__ folder2  
&nbsp; &nbsp; &nbsp; &nbsp; &nbsp; &nbsp; &nbsp; &nbsp; &nbsp; &nbsp;|__ request4  
&nbsp; &nbsp; &nbsp; &nbsp; &nbsp; &nbsp; &nbsp; &nbsp; &nbsp; &nbsp;|__ request5  
            
* Bombardier currently cannot generate different loads for different folders under collection. Whole collection will be executed with same thread count
* Bombardier cannot parse or execute Postman's javascript written under `test` tag. Due to this limitation you should explicitly tell bombardier if you wish to extract any value from response to be used in following requests. Refer [postprocessor](https://github.com/coding-yogi/bombardier/blob/develop/docs/postprocessor.md) guide for the same
