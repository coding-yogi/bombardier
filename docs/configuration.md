# Configuration

Below is the explanation of every config parameter mentioned in the config yaml  
  
|Parameter name                |Description                                                                             |Mandatory?                         |Default    |
|------------------------------|----------------------------------------------------------------------------------------|-----------------------------------|-----------|
|threadCount                   |No. of concurrent threads/users                                                         |No                                 |1          |
|iterations                    |No. of iterations every thread. Will supersede execution time if both are provided      |Yes (If execution_time == 0)       |           | 
|thinkTime                     |Time delay in ms between two consecutive requests on same thread                        |No                                 |1 ms       |
|executionTime                 |Execution time in secs                                                                  |No                                 |           |
|rampUpTime                    |Ramp up time in secs for starting all the threads                                       |No                                 |1 sec      |
|continueOnError               |Whether to continue with iteration when one of the requests fail                        |No                                 |false      |
|handleCookies                 |Whether bombardier will handle cookies. Mainly used in UI flow                          |No                                 |false      |
|database: type                |Type of storage used for real time stats, currently only influxdb is supported          |No                                 |           |
|database: url                 |Connection URL                                                                          |No                                 |           |
|database: user                |Database username                                                                       |No                                 |           |
|database: password            |Database password                                                                       |No                                 |           |
|database: name                |Database Name                                                                           |Yes (if url is provided)           |           |
|ssl: ignoreSSL                |Turn off SSL verification. Note: Disabling SSL verification is dangerous                |No                                 |false      |
|ssl: acceptInvalidHostnames   |Turn off host verification. Note: Disabling SSL host verification is dangerous          |No                                 |false      |
|ssl: certificate              |CA certificate file path (.pem or .der) that should be added to trust store             |No                                 |           |
|ssl: keystore                 |Key store file path having format .p12 or pfx                                           |No                                 |           |
|ssl: keystorePassword         |Password for the .p12 or pfx file specified as keystore                                 |No                                 |           |