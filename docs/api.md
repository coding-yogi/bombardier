# API Documentation  

## Get Nodes
### Gets the current number of connected nodes and bombarding nodes
```
curl --location --request GET 'http://localhost:9000/bombardier/v1/nodes'
```

Output: 200
```
{
    "available_nodes": 3,
    "bombarding_nodes": 0
}
```
if bombarding nodes are greater than 0 , that means execution is in progress  

## Start Execution
### Starts the distributed bombarding from all available nodes
```
curl --location --request POST 'http://localhost:9000/bombardier/v1/bombard' \
--form 'scenarios=@"/Users/ag/bombardier/examples/scenarios.yml"' \
--form 'environment=@"/Users/ag/bombardier/examples/dev.yml"' \
--form 'config=@"/Users/ag/bombardier/config.yml"' \
--form 'data="path_of_data_csv_file_on_node"' \
```

Output: 201
```
{
    "message": "execution started successfully"
}
```