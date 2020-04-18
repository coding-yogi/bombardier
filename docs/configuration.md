# Configuration

Below is the explanation of every config parameter mentioned in the config json  
  
|Parameter name    |Description                                                                         |Optional?  |Default    |
|------------------|------------------------------------------------------------------------------------|-----------|-----------|
|environment_file  |Path of the environment json exported from postman                                  |Yes        |           |
|collection_file   |Path of the collection json exported from postman                                   |No         |           |
|data_file         |Path of the data csv file to be used during execution                               |Yes        |           |
|thread_count      |No. of concurrent threads/users                                                     |Yes        |1          |
|iterations        |No. of iterations every thread. Will supersede execution time if both are provided  |Yes        |           | 
|thread_delay      |Time delay in ms between two consecutive requests on same thread                    |Yes        |1 ms       |
|execution_time    |Execution time in secs                                                              |Yes        |           |
|rampup_time       |Ramp up time in secs for starting all the threads                                   |Yes        |1 sec      |
|report_file       |Path to CSV report file                                                             |Yes        |report.csv |
|continue_on_error |Whether to continue with iteration when one of the requests fail                    |Yes        |false      |
|handle_cookies    |Whether bombardier will handle cookies. Mainly used in UI flow                      |Yes        |false      |
|influxdb          |If stats need to be pushed to influxDb for real time monitoring                     |Yes        |false      |