# Postprocessor

# Why is this required?
Many a times a set of requests form a flow. They need to execute in a sequence and the some of the requests may need an input from the responses of previous requests.  
  
For example, Making a token call and then using that token in following request. This would need extraction of token from token call, store it in memory and then use it in following request.  

This is exactly what postprocessor does, With some instructions, it lets you define what you need to extract from the response so that the extracted data can be used in following request.  

# Challenges
Postman already has something which can do this. It has a test script section where you can write some javascript to extract the data you need and store it in environments variable.  
  
Bombardier is written in Rust and cannot interpret the javascript written in postman so it needs some other way to know what is to be extracted.  

Whole idea of building bombardier was to use postman collection as it is. Asking user to make any changes to exported JSON and add additional instructions would defeat the purpose of bombardier. Also with this approach user would need to keep 2 copies of json, one for postman and one for bombardier.  

There was a solution needed where bombardier can be given instructions w/o editing the exported collection file, hence the below approach

# Defining post processor
Post processor can still be defined in `test` tag of postman. It would be a simple json variable with name `bombardier` and it wouldn't affect your existing test scripts.  
  
Currently post processing can be done for `json`, `xml` and `regex`, plan is to add `css` extractor as well.  
Below is an example of how a postprocessor json would look like. Please note that you would never need to define `xml` and `json` post processor for same request. Your response would either be a `json` or `xml`

```
var bombardier = {
    "gjson_path": {
        "foo1": "args.foo1",
        "foo2": "args.foo2",
    },
    "xpath": {
        "foo3": "//foo3/text()"
    },
    "regex": {
        "foo4": "\d+{8}"
    }
}
``` 

To know more about how to define gjson path you can refer [Gjson path](https://github.com/tidwall/gjson). There are many resources online to learn about xpaths and regexs.  
  
During execution, bombardier is only going to look for this particular json, it would ignore everything else written in the test script section.  
  
Bombardier will store the value extracted in the parameter with same name as the key. for eg. value extracted with gjson path `args.foo1` will be stored in a variable `foo1`. This variable then can later be used in following requests using double curly braces like https://some.url/?foo1={{foo1}}  
This is exactly the same way how you would have used it in postman but now instead of using postman's javascript here we are using bombardier variable to do the same