
# %%
pip install -e .
# %%
from postprocess import function as f

# %%
G, t_to_logic, logic_to_t = f.graph_from_runs("../data/**/*.csv")

# %%
import networkx as nx

playthrough = f.hashed_playthrough(
    "../data/normal_runs/RUNS/Open - 20211211_095828.csv"
)
print(len(playthrough))

prev_check_tile = None
prev_check_idx = None
errors = 0
possible_time_save = 0
better_routing_time_save = 0
with open("playthrough.txt", "w") as fl:
    for idx, tile in enumerate(playthrough):
        # check route from previous check to current check, excluding tile of current check
        # The time to the room of the current check is what's interesting (OF COURSE THERE ARE EDGE CASES AJGDKLDSJGL)
        if tile._row["check_ocurred"]:
            if prev_check_tile:
                try:
                    duration = (
                        playthrough[idx - 1].dt - prev_check_tile.dt
                    ).total_seconds()
                    path = " -> ".join(
                        [t.tile_name for t in playthrough[prev_check_idx:idx] + [tile]]
                    )

                    output_str = f"{path}, time: {duration}"
                    print(output_str)
                    fl.write(output_str + "\n")
                    shortest_path, shortest_time = G.dijkstra(
                        prev_check_tile.id, playthrough[idx - 1].id
                    )

                    shortest_path = " -> ".join(
                        [f.id_to_name()[logic_to_t[ltid]] for ltid in shortest_path]
                        + [tile.tile_name]
                    )
                    output_str = f"{shortest_path}, time: {shortest_time}"
                    print(output_str)
                    fl.write(output_str + "\n")
                    possible_time_save += duration - shortest_time
                    if path != shortest_path:
                        better_routing_time_save += duration - shortest_time

                except nx.NetworkXNoPath:
                    fl.write("ERROR\n")
                    errors += 1
            prev_check_tile = tile
            prev_check_idx = idx
    fl.write(f"Possible time save: {possible_time_save:.2f} seconds\n")
    fl.write(
        f"Possible time save from better routing: {better_routing_time_save:.2f} seconds\n"
    )

print("errors:", errors)

# %%
