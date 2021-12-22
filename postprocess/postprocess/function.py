from __future__ import annotations
from collections import defaultdict
import json
from pathlib import Path
import re
from typing import Dict, Iterable, Iterator, List, Sequence, Set, Tuple
from dataclasses import dataclass
from networkx.exception import NetworkXNoPath
import pandas as pd
import networkx as nx
from pandas import DataFrame

from itertools import takewhile

from pandas.core.series import Series

PACKAGE_ROOT = Path(__file__).parent
DATA_DIR = PACKAGE_ROOT / "data"

ITEM_NEVER_FOUND = 100_000_000

RUN_START = 20_000
RUN_END = 20_001

ITEM_IDS = {
    "sword": 27,
    "hammer": 13,
    "lantern": 12,
    "firerod": 7,
    "icerod": 8,
    "boots": 23,
    "glove": 24,
    "mirror": 22,
    "bow": 0,
    "hookshot": 4,
    "byrna": 21,
    "cape": 52,
    "pearl": 26,
    "bombos": 9,
    "flippers": 25,
}


def open_checks():
    with open(DATA_DIR / "checks.json") as f:
        return json.load(f)


def save_checks(checks: dict):
    with open(DATA_DIR / "checks.json", "w") as f:
        return json.dump(checks, f, indent=4)


def open_items():
    with open(DATA_DIR / "items.json") as f:
        return json.load(f)


def save_items(items: dict):
    with open(DATA_DIR / "items.json", "w") as f:
        return json.dump(items, f, indent=4)


def open_tiles():
    with open(DATA_DIR / "tiles.json") as f:
        return json.load(f)


def save_tiles(tiles: dict):
    with open(DATA_DIR / "tiles.json", "w") as f:
        return json.dump(tiles, f, indent=4)


def open_events():
    with open(DATA_DIR / "events.json") as f:
        return json.load(f)


def save_events(checks: dict):
    with open(DATA_DIR / "events.json", "w") as f:
        return json.dump(checks, f, indent=4)


def find_tiles(pattern: str):
    return [tile for tile in open_tiles() if re.match(pattern, tile["name"].lower())]


def find_items(pattern: str):
    return [item for item in open_items() if re.match(pattern, item["name"].lower())]


def item_log_idx(df: DataFrame, item: str, progressive_level: int = 1) -> int:
    found_items = df[df.item_id == ITEM_IDS[item]].index
    return (
        found_items[progressive_level - 1]
        if len(found_items) >= progressive_level
        else ITEM_NEVER_FOUND
    )


def can_slash(df: DataFrame) -> DataFrame:
    df["can_slash"] = df.index >= item_log_idx(df, "sword")
    return df


def can_hammer_things(df: DataFrame) -> DataFrame:
    df["can_hammer"] = df.index >= item_log_idx(df, "hammer")
    return df


def can_dash(df: DataFrame) -> DataFrame:
    df["can_dash"] = df.index >= item_log_idx(df, "boots")
    return df


def can_shoot(df: DataFrame) -> DataFrame:
    df["can_shoot"] = df.index >= item_log_idx(df, "bow")
    return df


def can_lift_rocks(df: DataFrame) -> DataFrame:
    df["can_lift_rocks"] = df.index >= item_log_idx(df, "glove")
    return df


def can_lift_heavy_rocks(df: DataFrame) -> DataFrame:
    df["can_lift_heavy_rocks"] = df.index >= item_log_idx(df, "glove", 2)
    return df


def can_remain_link_in_dw(df: DataFrame) -> DataFrame:
    df["can_remain_link_in_dw"] = df.index >= item_log_idx(df, "pearl")
    return df


def can_burn_things(df: DataFrame) -> DataFrame:
    df["can_burn_things"] = df.index >= item_log_idx(df, "firerod")
    return df


def can_melt_things(df: DataFrame) -> DataFrame:
    can_melt_idx = min(item_log_idx(df, "firerod"), item_log_idx(df, "bombos"))
    df["can_melt_things"] = df.index >= can_melt_idx
    return df


def can_light_things(df: DataFrame) -> DataFrame:
    can_light_idx = min(item_log_idx(df, "firerod"), item_log_idx(df, "lantern"))
    df["can_light_things"] = df.index >= can_light_idx
    return df


def can_traverse_big_gaps(df: DataFrame) -> DataFrame:
    df["can_traverse_big_gaps"] = df.index >= item_log_idx(df, "hookshot")
    return df


def can_swim(df: DataFrame) -> DataFrame:
    df["can_swim"] = df.index >= item_log_idx(df, "flippers")
    return df


def can_pass_energy_barriers(df: DataFrame) -> DataFrame:
    can_pass_idx = min(
        item_log_idx(df, "sword", progressive_level=2),
        item_log_idx(df, "byrna"),
        item_log_idx(df, "cape"),
    )
    df["can_pass_energy_barriers"] = df.index >= can_pass_idx
    return df


def all_eq_logic(dfs: Iterable[DataFrame]) -> Iterator[DataFrame]:
    for df in dfs:
        df = can_slash(df)
        # df = can_hammer_things(df)
        df = can_dash(df)
        # df = can_shoot(df)
        # df = can_lift_rocks(df)
        df = can_lift_heavy_rocks(df)
        df = can_remain_link_in_dw(df)
        # df = can_burn_things(df)
        # df = can_melt_things(df)
        # df = can_light_things(df)
        df = can_traverse_big_gaps(df)
        # df = can_swim(df)
        yield df


def create_row_hash(row: pd.Series | dict, cols=None):
    if not cols:
        cols = row.keys()

    key_tuple = ()
    for c in cols:
        val = row[c]
        if isinstance(val, list):
            key_tuple += (",".join(val),)
        else:
            key_tuple += (val,)

    return hash(key_tuple)


def convert_legacy_csv(df: DataFrame) -> DataFrame:
    if "transition_id" in df.columns:
        df["tile_id"] = df["transition_id"]
        df = df.drop(columns=["transition_id"])
    return df


def read_meta(input_file: str) -> dict:
    with open(input_file) as f:
        meta_lines = takewhile(
            lambda line: line.startswith("#") or line.strip() == "", f.readlines()
        )
    meta_lines = [line for line in meta_lines if line.strip() != ""]
    keyval_lines = (
        line.replace("#", "").strip().split(" ", maxsplit=1) for line in meta_lines
    )
    return {kv[0]: kv[1] for kv in keyval_lines}


def lines_of_meta(input_file: str) -> int:
    with open(input_file) as f:
        return len(
            [
                line
                for line in takewhile(
                    lambda line: line.startswith("#") or line.strip() == "",
                    f.readlines(),
                )
            ]
        )


def add_time_deltas(dfs: Iterable[DataFrame]) -> Iterator[DataFrame]:
    for df in dfs:
        start_time = df["timestamp"].min()
        df["timestamp"] = df["timestamp"] - start_time
        df["time_delta"] = df["timestamp"] - df["timestamp"].shift(1, fill_value=0)
        yield df


def read_run(path: Path | str) -> DataFrame:
    path = Path(path)
    print(f"Reading run {path}")
    if metalines := lines_of_meta(path):
        df = pd.read_csv(path, skiprows=metalines)
    else:
        df = pd.read_csv(path)
    df = convert_legacy_csv(df)
    df["filename"] = path.name
    return df


def read_runs(*glob_paths: List[str]) -> Iterator[DataFrame]:
    for glob in glob_paths:
        for path in Path.cwd().glob(glob):
            yield read_run(path)


def add_previous_and_future_tiles(dfs: Iterable[DataFrame]) -> Iterator[DataFrame]:
    """Used to create a `route_hash`"""
    for df in dfs:
        df["prevprevious_tile"] = df.tile_id.shift(2, fill_value=RUN_START)
        df["previous_tile"] = df.tile_id.shift(1, fill_value=RUN_START)
        df["next_tile"] = df.tile_id.shift(-1, fill_value=RUN_END)
        df["next_next_tile"] = df.tile_id.shift(-2, fill_value=RUN_END)
        yield df


SQ_TILE_ID = 30000
SQ_ID = 0
DEATH_TILE_ID = 40000
DEATH_ID = 1
RESET_TILE_ID = 50000
RESET_ID = 15


def add_special_tiles(dfs: Iterable[DataFrame]) -> Iterator[DataFrame]:
    for df in dfs:
        df.loc[df.event_id == SQ_ID, ["tile_id"]] = SQ_TILE_ID
        df.loc[df.event_id == RESET_ID, ["tile_id"]] = RESET_TILE_ID

        # Remove deaths as they themselves does not indicate a transition.
        df = df[df.event_id != DEATH_ID]
        df = df[df.tile_id.notnull()]
        yield df


def add_time_to_next(dfs: Iterable[DataFrame]) -> Iterator[DataFrame]:
    dfs = add_time_deltas(dfs)
    for df in dfs:
        # the next tiles timedelta is what actually presents time it takes to traverse current tile
        df.time_delta = df.time_delta.shift(-1, fill_value=0)
        yield df


_id_to_name = None


def id_to_name():
    global _id_to_name
    if not _id_to_name:
        json_tiles = open_tiles()
        id_to_name = {tile["id"]: tile["name"] for tile in json_tiles}
        # Names for special logic tiles that only exist in post processing
        id_to_name[RUN_START] = "RUN_START"
        id_to_name[RUN_END] = "RUN_END"
        id_to_name[RESET_TILE_ID] = "RESET"
        id_to_name[SQ_TILE_ID] = "S&Q"
        _id_to_name = id_to_name
    return _id_to_name


_name_to_id = None


def name_to_id():
    global _name_to_id
    if not _name_to_id:
        json_tiles = open_tiles()
        name_to_id = {tile["name"]: tile["id"] for tile in json_tiles}
        name_to_id["RUN_START"] = RUN_START
        name_to_id["RUN_END"] = RUN_END
        name_to_id["RESET"] = RESET_TILE_ID
        name_to_id["S&Q"] = SQ_TILE_ID
        _name_to_id = name_to_id
    return _name_to_id


from datetime import datetime as dt


@dataclass
class LogicTile:
    _row: dict
    _id: int = None
    _next_logical_tile: int = None

    def __hash__(self):
        return self.id

    def __eq__(self, other):
        if not isinstance(other, LogicTile):
            return False
        return self.id == other.id

    def __str__(self):
        return f"""\
id: {self.id}
tile_id: {self.tile_id}
tile_name: {self.tile_name}
next_logical_tile: {self.next_logical_tile}
link's abilities: {self.link_abilities()}
"""

    @property
    def id(self):
        if not self._id:
            self._id = create_row_hash(
                self._row,
                cols=["previous_tile", "tile_id", "next_tile"]
                + sorted([c for c in self._row if c.startswith("can_")]),
            )
        return self._id

    @property
    def next_logical_tile(self):
        """Returns the id of the next logical_tile, which should be a hash of:

        - Link's abilities on this tile (can_dash, can_slash etc)
        - this tiles `tile_id` as `previous_tile`
        - this tiles `next_tile` as `tile_id`
        - this tiles `next_next_tile` as `next_tile`

        Can be used in graph to make 'logic routes' (better name for this?)
        """
        if not self._next_logical_tile:
            self._next_logical_tile = create_row_hash(
                self._row,
                cols=["tile_id", "next_tile", "next_next_tile"]
                + sorted([c for c in self._row if c.startswith("can_")]),
            )
        return self._next_logical_tile

    @property
    def tile_id(self):
        return self._row["tile_id"]

    @property
    def tile_name(self):
        return id_to_name()[self._row["tile_id"]]

    @property
    def weight(self):
        return self._row["time_delta"]

    @property
    def dt(self):
        return dt.fromtimestamp(self._row["timestamp"] / 1000)

    def link_abilities(self):
        return ", ".join(
            [c for c in self._row if c.startswith("can_") and self._row[c]]
        )


def logic_tiles_from(dfs: Iterable[DataFrame]) -> Iterator[LogicTile]:
    dfs = all_eq_logic(dfs)
    dfs = add_special_tiles(dfs)
    dfs = add_previous_and_future_tiles(dfs)
    dfs = add_time_to_next(dfs)
    for df in dfs:
        for _, row in df.iterrows():
            yield LogicTile(dict(row))


@dataclass
class EdgeAttributes:
    source: int
    target: int
    weight: float


def adjacent_tiles_map(tiles: Iterable[LogicTile]) -> Dict[str, List[EdgeAttributes]]:
    """tiles: tiles taken from 1..n playthroughs of randomizer"""
    adjacents = defaultdict(dict)
    for tile in tiles:
        # Build up all edges for tiles
        tile_adjacents = adjacents[tile.id]

        # Keep the lowest time_delta on duplicate
        if existing_edge := tile_adjacents.get(tile.next_logical_tile):
            existing_edge.weight = min(existing_edge.weight, tile.weight)
        else:
            tile_adjacents[tile.next_logical_tile] = EdgeAttributes(
                tile.id, tile.next_logical_tile, tile.weight
            )
        adjacents[tile.id] = tile_adjacents
    return adjacents


def tile_to_logical_tiles(tiles: Iterable[LogicTile]) -> Dict[int, Set[int]]:
    tile_to_logical_tiles_map = defaultdict(set)
    for tile in tiles:
        tile_to_logical_tiles_map[tile.tile_id].add(tile.id)
    return tile_to_logical_tiles_map


def logical_tile_to_tile(tiles: Iterable[LogicTile]) -> Dict[int, int]:
    return {tile.id: tile.tile_id for tile in tiles}


def edges_from(
    adjacents: Dict[int, Dict[int, EdgeAttributes]]
) -> Iterator[EdgeAttributes]:
    for _, adjacent_tiles in adjacents.items():
        yield from adjacent_tiles.values()


def graph_from_runs(
    *glob_paths,
) -> Tuple[GraphWrapper, Dict[int, Set[int]], Dict[int, int]]:
    dfs = read_runs(*glob_paths)
    tiles = list(logic_tiles_from(dfs))
    nodes = [(tile.id, {"attrs": tile}) for tile in tiles]
    edges = edges_from(adjacent_tiles_map(tiles))

    G = nx.DiGraph()
    G.add_nodes_from(nodes)
    for edge in edges:
        try:
            G.add_edge(edge.source, edge.target, weight=edge.weight)
        except ValueError:
            raise ValueError(f"Failure when adding edge from EdgeAttributes: {edge}")

    return GraphWrapper(G), tile_to_logical_tiles(tiles), logical_tile_to_tile(tiles)


def node_id(name_or_id: int | str) -> int:
    if isinstance(name_or_id, str):
        return tile_name_to_id[name_or_id]
    return name_or_id


def tile_id_to_name(tile_id: float | int) -> str:
    """Maps tile_id to its human readable name.

    Handles 'special tiles' as well that are just logic tiles created in postprocessing"""
    return id_to_name()[int(tile_id)]


def tile_name_to_id(name: str) -> int:
    """Maps tile name to tile_id.

    Handles 'special tiles' as well that are just logic tiles created in postprocessing"""
    return name_to_id()[name]


def named_route(route: List[int]) -> List[tuple]:
    return [(id_, id_to_name()[id_]) for id_ in route]


def add_check_ocurred_on_tile(dfs: Iterable[DataFrame]) -> Iterator[DataFrame]:
    for df in dfs:
        df["check_ocurred"] = 0
        previous_row_idx = None
        tile_rows = df[(df["tile_id"].notnull())]
        for idx, row in tile_rows.iterrows():
            # print(row)
            if previous_row_idx is not None:
                check_ocurred = df[previous_row_idx:idx].location_id.notnull().any()
                df.loc[
                    df.index == previous_row_idx,
                    "check_ocurred",
                ] = int(check_ocurred)
            previous_row_idx = idx
        yield df


def hashed_playthrough(path) -> List[LogicTile]:
    df = read_run(path)
    dfs = all_eq_logic([df])
    dfs = add_check_ocurred_on_tile(dfs)
    dfs = add_special_tiles(dfs)
    dfs = add_previous_and_future_tiles(dfs)
    df = next(add_time_to_next(dfs))
    return [LogicTile(dict(row)) for _, row in df.iterrows()]


@dataclass
class GraphWrapper:
    g: nx.DiGraph

    def dijkstra(self, source: int, target: int, weight="weight"):
        """returns the shortest path in time between `source` and `target`

        `source` and `target` can be either a `tile_id: int`, or a `name: str`. the method will convert the values correctly"""
        try:
            time = nx.dijkstra_path_length(self.g, source, target, weight=weight) / 1000
        except NetworkXNoPath:
            raise NetworkXNoPath(
                f"Node {self.g.nodes[target]['attrs']} not reachable from {self.g.nodes[source]['attrs']}"
            )
        route = nx.dijkstra_path(self.g, source, target, weight=weight)
        return (route, time)

    def all_shortest_paths(
        self, source: int | str, target: int | str, weight="weight", method="dijkstra"
    ):
        for path in nx.all_shortest_paths(
            self.g, node_id(source), node_id(target), weight=weight, method=method
        ):
            for edge in path:
                self.g.edges[edge]["weight"]
            yield [id_to_name()[p] for p in path]
