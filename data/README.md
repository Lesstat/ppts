# Data Description
This directory contains the graph data used for the paper titled _Scalable Unsupervised Multi-Criteria Trajectory Segmentation and Driving Preference Mining_ [1]. The data includes a graph representation of the most important roads in the Danish road network, four different edge traversal costs, and the two cost-annotated graphs used in our experiments. In addition, the data includes synthetic trajectories for running our algorithm.

## Decompressing the Dataset
The dataset is contained in three subdirectories: `graphs`, `costs`, and `cost_annotated_graphs`. The contents of these subdirectories are compressed in ZIP format. Unzip the files `graphs/graphs.zip`, `costs/costs.zip`, `cost_annotated_graphs/cost_annotated_graphs.zip`, and `cost_annotated_graphs/cost_annotated_graphs_without_normalization.zip` to decompress the dataset. For convenience, we have included the shell script `decompress_data.sh` which unzips all the mentioned files.

## Subdirectory Contents
After decompression of the dataset, the `graphs` folder contains the file `denmark.graphml`, a graph representation of the most important roads in the Danish road network in GraphML format. In this graph representation, the nodes represent intersections or the end of a road, and edges represent road segments.

The `costs` folder contains the files `congestion_denmark.costs.json`, `travel_time_denmark.costs.json`, `unit_distance_denmark.costs.json`, and `crowdedness_denmark.costs.json`. These files contains traversal costs for the road segments in the representation of the Danish road network (`graphs/denmark.graphml`). The files are stored as dictionaries in a JSON format where the keys correspond to road segment names, i.e., the values of the _name_ edge attribute in the graph representation of the Danish road network. See [1] for more details on the dataset.

The `cost_annotated_graphs` directory contains two cost annotated graphs
 - `denmark_with_normalized_costs_travel_time_crowdedness_congestion_unit_distance.graphml` 
 - `denmark_with_normalized_costs_travel_time.graphml`
which, as implied by their names, are graph representations of the Danish road network that are annotated with all four costs and only the travel time cost, respectively, as edge attributes. The costs are normalized so they have a mean of 1, but for interpretability we have included the files
 - `denmark_with_costs_travel_time_crowdedness_congestion_unit_distance.graphml` 
 - `denmark_with_costs_travel_time.graphml`
containing cost annotated graphs where the costs are not normalized.

## References
[1] Barth, F., Funke, S., Jepsen, T.S., and Proissl, C. Scalable Unsupervised Multi-Criteria Trajectory Segmentation and Driving Preference Mining. In _Proceedings of the 28th ACM SIGSPATIAL International Conference on Advances in Geographic Information Systems_. 2020.
