# %%

import pandas as pd
import numpy as np
from pandas import DataFrame
from typing import List, Callable
from typing import Iterator
from numpy import int64
from datetime import datetime
from typing import List

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

RUN_START = 20_000
RUN_END = 20_001

# Value that will return false for all relevant checks
ITEM_NEVER_FOUND = 100_000_000

# Column names
ROUTE_HASH = "route_hash"
TILE_EVENTS_HASH = "tile_events_hash"
ABILITY_HASH = "ability_hash"
LOGIC_ROUTE_HASH = "logic_route_hash"
TILE_ID = "tile_id"
EVENT_ID = "event_id"
LOCATION_ID = "location_id"
ITEM_ID = "item_id"
TIMESTAMP = "timestamp"


# %%
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


def all_eq_logic(df: DataFrame) -> DataFrame:
    df = can_slash(df)
    df = can_hammer_things(df)
    df = can_dash(df)
    df = can_shoot(df)
    df = can_lift_rocks(df)
    df = can_lift_heavy_rocks(df)
    df = can_remain_link_in_dw(df)
    df = can_burn_things(df)
    df = can_melt_things(df)
    df = can_light_things(df)
    df = can_traverse_big_gaps(df)
    df = can_swim(df)
    return df


def create_row_hash(row: pd.Series):
    def kv_str(key, val):
        return f"{key}:{str(val)}"

    key_tuple = ()
    for key, val in sorted(row.items()):
        if isinstance(val, list):
            key_tuple += (",".join(kv_str(key, item) for item in val),)
        else:
            key_tuple += (kv_str(key, val),)

    return hash(key_tuple)


def apply_current_abilities(
    df: DataFrame, eq_logic: List[Callable[[DataFrame], DataFrame]] = None
) -> DataFrame:
    if not eq_logic:
        df = all_eq_logic(df)
    else:
        for eq_func in eq_logic:
            df = eq_func(df)
    ability_columns = [c for c in df.columns if c.startswith("can_")]
    df[ABILITY_HASH] = df[ability_columns].apply(create_row_hash, axis=1)

    return df


def only_tiles(df: DataFrame) -> DataFrame:
    return df[df.tile_id.notna()].drop(columns=["location_id", "item_id", "event_id"])


def add_previous_and_future_tiles(df: DataFrame) -> DataFrame:
    """Used to create a `route_hash`"""
    df["previous_tile"] = df.tile_id.shift(1, fill_value=RUN_START)
    df["next_tile"] = df.tile_id.shift(-1, fill_value=RUN_END)
    df["next_next_tile"] = df.tile_id.shift(-2, fill_value=RUN_END)
    return df


def apply_routes_hash(df: DataFrame) -> DataFrame:
    """Create a unique 'route id' from all columns named something with `tile`"""
    df = add_previous_and_future_tiles(df)
    tile_columns = sorted([c for c in df.columns if "tile" in c])
    df[ROUTE_HASH] = df[tile_columns].apply(create_row_hash, axis=1)
    return df


def apply_tile_events_hash(df: DataFrame) -> DataFrame:
    """Create a unique 'tile events id' from columns `item_id, location_id, event_id`"""
    df = add_previous_and_future_tiles(df)
    columns_of_interest = sorted([ITEM_ID, LOCATION_ID, EVENT_ID])
    df[TILE_EVENTS_HASH] = df[columns_of_interest].apply(create_row_hash, axis=1)
    return df


def add_time_deltas(df: DataFrame) -> DataFrame:
    start_time = df[TIMESTAMP].min()
    df[TIMESTAMP] = df[TIMESTAMP] - start_time
    df["time_delta"] = df[TIMESTAMP] - df[TIMESTAMP].shift(1, fill_value=0)
    return df


def apply_logic_route_hash(
    df: DataFrame, columns_to_hash: List[str] = None
) -> DataFrame:
    """Combine all hashes into a  single unique identifier for the given 'logical route' """
    if not ABILITY_HASH in df.columns:
        raise AttributeError(
            f"apply_logic_route_hash: {ABILITY_HASH} needs to exist before running this function"
        )
    if not all(df.tile_id.notnull()):
        raise AttributeError(
            f"apply_logic_route_hash: every df row needs to have a {TILE_ID} before running this function"
        )
    if not ROUTE_HASH in df.columns:
        df = apply_routes_hash(df)
    if not columns_to_hash:
        columns_to_hash = sorted(c for c in df.columns if "hash" in c)
    df[LOGIC_ROUTE_HASH] = df[sorted(columns_to_hash)].apply(create_row_hash, axis=1)
    return df


def convert_legacy_csv(df: DataFrame) -> DataFrame:
    if "transition_id" in df.columns:
        df["tile_id"] = df["transition_id"]
        df = df.drop(columns=["transition_id"])
    return df


from pathlib import Path
from typing import List


def read_runs(
    glob_pattern: str,
    eq_logic: List[Callable[[DataFrame], DataFrame]] = None,
    columns_for_superhash: list = None,
):
    """Attempts reading all files matching as csv, and combines them into one single dataframe.

    All rows where `tile_id == NaN` will be merged into last seen row with a tile_id.

    `item_id`, `location_id`, and `event_id` will be turned into lists containing 0..n items.

    `eq_logic` is a list of functions that take a pandas `DataFrame` as argument, and returns a `DataFrame`.
    If `None`, will use `all_eq_logic` function defined above."""
    dfs: List[DataFrame] = []
    if columns_for_superhash:
        print(f"Using {columns_for_superhash} for combined superhash")
    for path in Path.cwd().glob(glob_pattern):
        df = pd.read_csv(path)
        df = convert_legacy_csv(df)
        df = apply_current_abilities(df, eq_logic=eq_logic)
        # this needs to happen before apply_route_ability_hash so that every row has a new tile_id
        df = DataFrame(list(merge_tile_events(df)))
        df = apply_tile_events_hash(df)
        df = apply_logic_route_hash(df, columns_to_hash=columns_for_superhash)
        df = add_time_deltas(df)
        df["filename"] = path.name
        dfs.append(df)
    return pd.concat(dfs)


# %%
def new_tile_events(row):
    tile_events = {}
    tile_events[TIMESTAMP] = row[TIMESTAMP]
    try:
        tile_events[TILE_ID] = np.int64(row[TILE_ID])
    except ValueError as e:
        print(f"Row did not have a tile_id for some goddamn reason: {row}")
        raise e
    tile_events[LOCATION_ID] = []
    tile_events[ITEM_ID] = []
    tile_events[EVENT_ID] = []
    tile_events[ABILITY_HASH] = row[ABILITY_HASH]
    return tile_events


def merge_tile_events(df: DataFrame) -> Iterator[dict]:

    tile_events = {}

    for _, row in df.iterrows():
        # print(f"{idx}: {row.to_dict()}")
        if not tile_events:
            tile_events = new_tile_events(row)
        else:
            if not np.isnan(row[TILE_ID]) and tile_events[TILE_ID] != row[TILE_ID]:
                # print(f"Row {idx} is a new tile because {tile_events[TILE_ID]} != {row[TILE_ID]}")
                yield tile_events
                tile_events = new_tile_events(row)
            else:
                if row[EVENT_ID] is not None and not np.isnan(row[EVENT_ID]):
                    tile_events[EVENT_ID].append(int64(row[EVENT_ID]))
                if row[LOCATION_ID] is not None and not np.isnan(row[LOCATION_ID]):
                    tile_events[LOCATION_ID].append(int64(row[LOCATION_ID]))
                if row[ITEM_ID] is not None and not np.isnan(row[ITEM_ID]):
                    tile_events[ITEM_ID].append(int64(row[ITEM_ID]))
    # last tile_events row won't be yielded unless we do this
    if tile_events and tile_events[TILE_ID]:
        yield tile_events


# %%
def best_route_time(runs):
    def wrapped(row):
        return runs[runs[LOGIC_ROUTE_HASH] == row[LOGIC_ROUTE_HASH]].time_delta.min()

    return wrapped


class RunComparator:
    def __init__(self, all_runs: DataFrame) -> None:
        self.all_runs: DataFrame = all_runs
        self.best_route_time = best_route_time(self.all_runs)

    def apply_best_route_time(self, selected_run):
        rows = []
        for _, row in selected_run.iterrows():
            row["best_time_delta"] = self.best_route_time(row)
            rows.append(row)
        return DataFrame.from_records(rows)

    def best_possible_time_for(self, filename: str) -> DataFrame:
        selected_run = self.all_runs[self.all_runs.filename == filename]
        if len(selected_run) == 0:
            raise AttributeError(
                f"run {filename} does not exist. Possible values: {self.all_runs['filename'].unique()}"
            )
        else:
            processed = self.apply_best_route_time(selected_run)
            print("Run: ", filename)
            print(
                f"Total time: {datetime.fromtimestamp(processed.time_delta.sum()/ 1000) - datetime.fromtimestamp(0)}"
            )
            print(
                f"Best possible time: {datetime.fromtimestamp(processed.best_time_delta.sum()/1000) - datetime.fromtimestamp(0)}\n"
            )
            return processed

