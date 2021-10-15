# Benchmarks

Following tools were considered for benchmarking as they are pretty well know in the open source community

- Jmeter (Java)
- K6 (Go)
- Gatling (Scala)
- Locust (Python)

## Execution configuration
- Runner : Mac machine 2.6 GHz 6-Core Intel Core i7 with 16 GB memory 
- Flow: 5 requests as defined in Scenario section
- Total time of execution : 300 secs (5 mins)
- User load : 100 users
- Ramup time : 100 secs (1 user every sec)
- Data fed through CSV file containing 10k records
- All tools wrote the report to a CSV file

## Scenario
To keep test limited but realistic below is the scenario which was executed on all tools.  
1. GET call with JSON extraction
2. POST with URL encoded params with JSON extraction
3. POST with form data with JSON extraction
4. POST with multipart form data file upload with JSON extraction
5. POST with JSON with JSON extraction

```
version: 1.0
scenarios:
  - name: scenario1
    requests:
    - name: echoGet
      method: GET
      url: "{{baseUrl}}/get"
      headers:
        authorization: bearer jwt
      extractors:
        - from: Body
          type: GjsonPath
          extract:
            authHeader: "headers.Authorization"
            host: "headers.Host" 

    - name: echoPostWithUrlEncoded
      method: POST
      url: '{{baseUrl}}/post'
      body:
        urlencoded:
          key23: '{{Country}}'
          key24: '{{City}}'
      extractors:
        - type: GjsonPath
          extract:
            country: "form.key23"
            city: "form.key24"

  - name: scenario2
    requests:
    - name: echoPostFormData
      method: POST
      url: '{{baseUrl}}/post'
      body:
        formdata:
        - name: key1
          value: '{{AccentCity}}'
      extractors:
        - type: GjsonPath
          extract:
            keyname: "form.key1"

    - name: echoPostFormDataWithFile
      method: POST
      url: '{{baseUrl}}/post'
      body:
        formdata:
        - name: key21
          value: key22
        - name: file
          value: '/Users/coding-yogi/work/repos/bombardier/examples/configdev.json'
          type: File
      extractors:
        - type: GjsonPath
          extract:
            keyname: "files.file"

    - name: echoPostJSON
      method: POST
      url: '{{baseUrl}}/post'
      headers:
        content-type: application/json
      body:
        raw: |
          {
            "test": "test"
          }   
      extractors:
        - type: GjsonPath
          extract:
            keyname: "json.test"
```

## Benchmarks
Below benchmarks are the averges of 3 executions of every tool.   
`gtime` which is `time` equivalent for `OSX` was used to measure CPU and Mem utilization

| Tool | Version | CPU Usage | Memory Usage | Total requests | TPS |
|-------|-------|------|------|-----|-----|
| Bombardier (Rust) | 1.1 | **11.5%** | **26 MB** | **90339** | **301.13** |
| Jmeter (Java) | 5.4.1 | 17% | 870 MB | 89919 | 299.73 |
| K6 (Go) | 0.34.1 | 34% | 248 MB | 88119 | 293.73
| Locust (Python)| 2.4.0 | 45% | 143 MB | 79931 | 266.43 |
| Gatling (Scala) | 3.6.1 | 25% | 922 MB | 90159 | 300.53 |