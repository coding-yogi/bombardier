# Extractors
Extractors are basically post-processors which can parse the http response and can extract some relevant information from the reponse which then can be used in following requests.

Extraction of information from the response can be done from 2 sources
1. From response body
2. From response headers

For extraction from `body`, there are mainly 3 types of extractors supported as of now

- `GjsonPath` extractor
- `Xpath` extractor
- `RegEx` extractor

## GjsonPath
GJSON extractor works just like jsonpath. To know about syntax read [this](https://github.com/tidwall/gjson/blob/master/SYNTAX.md)

**Example :**
Received response:
```
{
  "args": {}, 
  "headers": {
    "Accept": "*/*", 
    "Authorization": "jwt 123", 
    "Host": "httpbin.org", 
    "X-Amzn-Trace-Id": "Root=1-6165831a-4bc7a6a769b2f47a0049a072"
  }, 
  "origin": "121.7.130.155", 
  "url": "https://httpbin.org/get"
}
```

**Defined extractor :**
```
extractors:
- from: Body
  type: GjsonPath
  extract:
    authHeader: "headers.authorization"
    host: "headers.host"
```

**Output**  
Environment map will have 2 new entries added
1. authHeader = jwt 123
2. host = httpbin.org

*Note:* GjsonPath extractors will only work for JSON responses

## Xpath 
As the name suggests this is an xpath based extractor and works for both XML as well as HTML responses  

**Example :**
Received response
```
<?xml version='1.0' encoding='us-ascii'?>
<!--  A SAMPLE set of slides  -->
<slideshow 
    title="Sample Slide Show"
    date="Date of publication"
    author="Yours Truly"
    >
    <!-- TITLE SLIDE -->
    <slide type="all">
        <title>Wake up to WonderWidgets!</title>
    </slide>
    <!-- OVERVIEW -->
    <slide type="all">
        <title>Overview</title>
        <item>Why 
            <em>WonderWidgets</em> are great
        </item>
        <item/>
        <item>Who 
            <em>buys</em> WonderWidgets
        </item>
    </slide>
</slideshow>
```

**Defined extractor :**
```
extractors:
- from: Body
  type: Xpath
  extract:
    slide_type: "/slideshow/slide/@type"
    title2: "/slideshow//title[2]"
```

**Output**  
Environment map will have 2 new entries added
1. slide_type = all
2. title2 = Overview

**Note:** XPATH extractor will only work for XML/HTML, else they will be ignored

## RegEx
Regex matches the specificed pattern and extracts the corresponding value from the whole response text.  
If capture group is specified then the match corresponding to 1st capture group is returned, else whole match is returned
    
Extractors are executed in the sequence they are specified.

Received response:
```
{
  "args": {}, 
  "headers": {
    "Accept": "*/*", 
    "Authorization": "jwt 123", 
    "Host": "httpbin.org", 
    "X-Amzn-Trace-Id": "Root=1-6165831a-4bc7a6a769b2f47a0049a072"
  }, 
  "origin": "121.7.130.155", 
  "url": "https://httpbin.org/get"
}
```

**Defined extractor:**
```
extractors:
- from: Body
  type: RegEx
  extract:
    amazonTraceID: "Root=.*"
    amazonTraceIDRootID: "Root=(.*)"
```

**Output**  
Environment map will have 2 new entries added
1. amazonTraceID = Root=1-6165831a-4bc7a6a769b2f47a0049a072
2. amazonTraceIDRootID = 1-6165831a-4bc7a6a769b2f47a0049a072 *(1st capture group is returned)*


## Extraction from response header
Here the `form` value in extractor is `Headers` instead of `Body`

**Example:**
Received response headers
```
Content-Type: application/xml
Server: gunicorn/19.9.0
Content-Length: 552
```

**Defined Extractor:**
```
extractors:
- from: Headers
  extract:
    content-type: "Content-Type"
    server: "Server"
```

**Output**  
Environment map will have 2 new entries added
1. content-type = application/xml
2. server = gunicorn/19.9.0