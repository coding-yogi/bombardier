# Bombardier ![bombardier](https://github.com/coding-yogi/bombardier/workflows/bombardier/badge.svg)
Rust based HTTP load testing tool using yaml files.  

Bombardier can take your simple yaml based files containing scenarios & environment variables and bombard your application with a defined load.  

Bombardier needs 2 files at minimum and 4 files at max to carry out any load tests
- `config.yaml` *(required)* - Contains the execution configuration like no of threads, rampup time etc. Check config section for more details
- `scenarios.yml` *(required)* - File containing scenarios which would be used to generate load
- `environments.yml` *(optional)* - Environment variables which would be replaced in scenarios file during execution
- `data.csv` *(optional)* - CSV file to supply test data

## Config yml
You need to create a yml file which can tell Bombardier about the load configuration.  

```
version: 1.0
threadCount: 1
iterations: 10
threadDelay: 1
executionTime: 0
rampUpTime: 1
continueOnError: true
handleCookies: false
ssl:
  ignoreSSL: true
  acceptInvalidHostnames: false
  certificate: "./ca_cert.pem"
  keyStore: "./keystore.p12"
  keyStorePassword: "P@$$w0rd123"
database:
  type: influxDB
  url: http://some-influxdb-url/
  name: dbName
  user: someUser
  password: P@$$w0rd123
```

For more details regarding configuration json, please check [configurations](docs/configuration.md) doc.  

## Scenarios file
A simple scenarios.yml looks something like below. Every scenario can have N requests.  
Each request is defined with its name, method, url, headers, body and extractors.    

Extractors is an array which are more like post processors to be applied on the received response, they help to extract certain values from response which then can be used in the following requests.  

To know more about extractors check [this](docs/extractors.md) doc
```
version: 1.0
scenarios:
  - name: scenario1
    requests:
    - name: echoGet
      method: GET
      url: "{{baseUrl}}/get"
      headers:
        authorization: jwt 123
      extractors:
        - type: gjsonpath
          extract:
            authHeader: "headers.authorization"
            host: "headers.host"
          
    - name: echoPostWithUrlEncoded
      method: POST
      url: '{{baseUrl}}/post'
      body:
        urlencoded:
          key23: value23
          key24: value24
      extractors:
        - type: gjsonpath
          extract:
            keyname1: "form.key23"
        - type: gjsonpath
          extract:
            keyname2: "form.key24"
```

## Environment file
Many a times there would be need to have some variables which needs to be used through the tests, One such example you can see in the above tests is the `url` value. As the baseURL would remain same, you would want to pull it out of the tests, so that it can be updated (if required) later at a single place. All such variables can go into a file `environment.yml`, Below is an example of the same
```
version: 1.0
variables:
  baseUrl: https://postman-echo.com
```

Bombardier will do the necessary replacement of the values from environment file into the scenarios file at runtime
  
## Data file
Data file is a simple CSV format file with 1st row as header values, Data is injected into the test by reading from csv file. 
At end of CSV file, tests start reading the data again from 1st row till the execution ends.

Similar to environments file and data which is read from the data file will be replaced as per the parameter name.
Parameter name should match the column name in the csv file for successful substitution of the value 
  
## Building from source  
Make sure you have cargo and rust installed. Checkout the code and run below command.  
If code builds successfully you should see the binary/executable in `/target/release` folder
  
`cargo build --release`  
  
## Troubleshooting
If you face issues building the binaries from the source please check [troubleshooting](docs/troubleshooting.md) document
  
## Using binaries
If you do not wish to build bombardier from source, you can always download the binaries and use the tool directly. Go to [releases](https://github.com/coding-yogi/bombardier/releases) to download the binary of your choice
  
## Using docker
Bombardier can run inside a container too. You can build the image with below command or just pull the `codingyogi/bombardier:latest` image from docker hub   
`docker build . -t bombardier`  

Container can be started using below command  
`docker run --name bombardier -v $PWD:/home/tests bombardier:latest bombard -c ./tests/config.yml -s ./tests/scenarios.yml -e ./tests/env.yml -r ./tests/report.csv`  

Note the volume used. Present working directory on host is mapped to `/home` directory on the container. 
With this approach you need not copy your config file or collections file into the container.
  
## Running Tests on a standalone machine
`./bombardier bombard -c <path of config yml> -s <path of scenarios yml> -e <path of env yml> -d <path of data csv>`

  
## Distributed Tests
Distributed tests run in a Control-Data plan architecture. Hub is a control plane from where you can control the execution.  
Nodes are part of data plane which are responsible for generating the actual load.  

### Starting bombardier as a hub  
 `./bombardier hub -p <port for rest server> -s <port for websocket server>`

 Hub is responsible for distributing load to all its nodes and it runs 2 servers  
 One is a REST server for user to interfact with bombardier, second is a websocket server for the hub to communicate with its nodes.
 Check [api](docs/api.md) documentation for API details
  

### Starting bombardier as a node (load generator) 
`./bombardier node -h <hub address>` 

Node requires hub's address to connect to, so make sure hub is up and running before nodes are started
  

## Enabling debug mode for more logs
`export RUST_LOG=debug`  
Debug logs would be written only to log file. It is not advisable to enable debug logging during actual execution of tests  
  

## Generating reports
`./bombardier report -r <path to csv report file>`  
  

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