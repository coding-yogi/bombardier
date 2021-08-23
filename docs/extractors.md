# Extractors
Extractors are basically post-processors which can parse the http response and can extract some relevant information from the reponse which then can be used in following requests.

Think of a scenario where you make a token call to get the Bearer token and then use the token as an authorization header in the following requests.  

There are mainly 3 types of extractors supported as of now

- GJSON extractor
- XPATH extractor
- REGEX extractor

## GJSON
GJSON extractor works just like jsonpath. To know about syntax check [this](https://github.com/tidwall/gjson/blob/master/SYNTAX.md)

## XPATH 
As the name suggests this is an xpath based extractor and works for both XML as well as HTML responses

## REGEX
Regex matches the specificed pattern and extracts the corresponding value from the whole response text.
    
Extractors are executed in the sequence they are specified.

## Example 
Received response:
```
{
    "args": {},
    "headers": {
        "host": "postman-echo.com",
        "authorization": "jwt 123",
    },
    "url": "https://postman-echo.com/get"
}
```

Here the `type` of extractor is define as `gjsonpath` that means it will use gjson extractor.  
It will extract `headers.authorization` from response body and add it into an env variable called `authHeader`
Similarly it will extract `headers.host` from the response and store it in `host` env variables.  
Later these variables can be used in following requests using `{{var_name}}` notation

```
extractors:
- type: gjsonpath
  extract:
    authHeader: "headers.authorization"
    host: "headers.host"
```

Note: GSON extractors will only work for JSON responses and XPATH extractor will only work for XML/HTML, else they will be ignored
