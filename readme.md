# Bombardier ![bombardier](https://github.com/coding-yogi/bombardier/workflows/bombardier/badge.svg)
Rust based HTTP load testing tool using postman collection  
Bombardier can take your existing postman collection and bombard your server with those requests under specified load  
  
## Building from source
Make sure you have cargo and rust installed. Checkout the code and run below command.  
If code builds successfully you should see the binary/executable in `/target/release` folder
  
`cargo build --release`  

## Troubleshooting
If you face issues building the binaries from the source please check [troubleshooting](docs/troubleshooting.md) document

## Using binaries
If you do not wish to build bombardier from source, you can always download the binaries and use the tool directly. Go to [releases](https://github.com/coding-yogi/bombardier/releases) to download the binary of your choice

## Using docker
Bombardier can run as a Docker container too. You can build the image with following command  
`docker build . -t bombardier`  

Container can be started using below command  
`docker run --name bombardier -v $PWD:/home bombardier:latest bombard --config /home/config.json`  

Note the volume used. Present working directory on host is mapped to `/home` directory on container. 
With this approach you need not copy your config file or collections file into the container. Make sure you update paths accordingly in config file
  
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
    "ssl": {
        "ignore_ssl" : false,
        "accept_invalid_hostnames": false,
        "certificate": "./ca_cert.pem",
        "keystore": "./keystore.p12",
        "keystore_password": "P@$$w0rd123"
    },
    "influxdb" : {
        "url": "http://localhost:8086",
        "username": "",
        "password": "",
        "dbname": "mydb"
    },
    "distributed": true,
    "nodes": [
        "10.16.21.98:7000",
        "10.16.21.99:7000",
        "10.16.21.100:7001"
    ]
}
```

For more details regarding configuration json, please check [configurations](docs/configuration.md) doc.  

## Running Tests on single instance
`./bombardier bombard --config <path of config json>`

## Distributed Tests
Starting bombardier as a node  
 `./bombardier node --port <port on which node should listen>`

Starting bombardier as a load distributor  
`./bombardier bombard --config <path of config json>` 

Note: Make sure `distributed` flag is set to `true` in config file and nodes are running

## Enabling debug mode for more logs
`export RUST_LOG=debug`  
Debug logs would be written only to log file. It is not advisable to enable debug logging during actual execution of tests  

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
  

## Benchmarks
I would like this tool to be benchmarked with other tools to see if it needs more improvement. You can find the benchmarks [here](docs/benchmarks.md)


## Limitations
* Bombardier currently will only parse the requests which are directly under collection folder or it's sub-folder. It will ignore requests from folders which are 2 or more levels down in hierarchy.
  In below example bombardier will ignore all requests under folder 2

```
├── collection
    ├── request1
    └── folder1
        ├── request2
        ├── request3
        └── folder2
            ├── request4
            └── request5
```  
            
* Bombardier currently cannot generate different loads for different folders under collection. Whole collection will be executed with same thread count
* Bombardier cannot parse or execute Postman's javascript written under `test` tag. Due to this limitation you should explicitly tell bombardier if you wish to extract any value from response to be used in following requests. Refer [postprocessor](docs/postprocessor.md) guide for the same
