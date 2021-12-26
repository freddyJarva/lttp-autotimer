# %%
pip install -e .
# %%
from postprocess import function as f

# %%
G, t_to_logic, logic_to_t = f.graph_from_runs("../data/runs/*.csv")

# %%
import networkx as nx

for node in G.g.nodes:
    n = G.g.nodes[node]
    if 'attrs' in n:    
        logtile = n['attrs']
        n['name']     = logtile.tile_name
        n['abilities'] = logtile.link_abilities()
        del n['attrs']

nx.write_graphml(G.g, 'out.graphml')