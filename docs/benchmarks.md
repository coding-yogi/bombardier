# Benchmarks

## Comparison with ab (apache benchmark)

Request made `https://postman-echo.com/get?foo1=bar1&foo2=bar2`  
Concurrency level : 10  
Requests made: 1000  
Keep Alive: yes  

During all the requests bombardier wrote to csv & also did some post processing on response  

Below table states time taken in secs to complete 1k requests   


| Runs|ab| bombardier|
|-----|--|-----------|
| run 1 | 30.385  | 28  |
| run 2 | 29.354  | 31  |
| run 3 | 28.846  | 28  |
