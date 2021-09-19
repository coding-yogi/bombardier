# Configuration

Below is the explanation of every config parameter mentioned in the config json  
  
|Parameter name                |Description                                                                             |Mandatory?                         |Default    |
|------------------------------|----------------------------------------------------------------------------------------|-----------------------------------|-----------|
|environment_file              |Path of the environment yaml (required for local execution)                             |No                                 |           |
|scenarios _file               |Path of the scenarios yaml (required for local execution)                               |Yes                                |           |
|data_file                     |Path of the data csv file to be used during execution (required for local execution)    |No                                 |           |
|thread_count                  |No. of concurrent threads/users                                                         |No                                 |1          |
|iterations                    |No. of iterations every thread. Will supersede execution time if both are provided      |Yes (If execution_time == 0)       |           | 
|think_time                    |Time delay in ms between two consecutive requests on same thread                        |No                                 |1 ms       |
|execution_time                |Execution time in secs                                                                  |No                                 |           |
|rampup_time                   |Ramp up time in secs for starting all the threads                                       |No                                 |1 sec      |
|report_file                   |Path to CSV report file                                                                 |No                                 |report.csv |
|continue_on_error             |Whether to continue with iteration when one of the requests fail                        |No                                 |false      |
|handle_cookies                |Whether bombardier will handle cookies. Mainly used in UI flow                          |No                                 |false      |
|influxdb                      |If stats need to be pushed to influxDb for real time monitoring                         |No                                 |false      |
|influxdb: url                 |Connection URL for influxDB                                                             |No                                 |           |
|influxdb: username            |DB username                                                                             |No                                 |           |
|influxdb: password            |DB password                                                                             |No                                 |           |
|influxdb: dbname              |Database Name                                                                           |Yes (if url is provided)           |           |
|ssl: ignore_ssl               |Turn off SSL verification. Note: Disabling SSL verification is dangerous                |No                                 |false      |
|ssl: accept_invalid_hostnames |Turn off host verification. Note: Disabling SSL host verification is dangerous          |No                                 |false      |
|ssl: certificate              |CA certificate file path (.pem or .der) that should be added to trust store             |No                                 |           |
|ssl: keystore                 |Key store file path having format .p12 or pfx                                           |No                                 |           |
|ssl: keystore_password        |Password for the .p12 or pfx file specified as keystore                                 |No                                 |           |