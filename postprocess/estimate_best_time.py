import pandas as pd
import numpy as np
from pandas import DataFrame
from pathlib import Path
from typing import List
from datetime import datetime

CHECKS = pd.read_json('src/checks.json')
DROPS = pd.read_json('src/drops.json')
EVENTS = pd.read_json('src/events.json')
ITEMS = pd.read_json('src/items.json')
TILES = pd.read_json('src/tiles.json')

ITEM_IDS = {
    'sword': 27,
    'hammer': 13,
    'lantern': 12,
    'firerod': 7,
    'icerod': 8,
    'boots': 23,
    'glove': 24,
    'mirror': 22,
    'bow': 0,
    'hookshot': 4,
    'byrna': 21,
    'cape': 52,
    'pearl': 26,
    'bombos': 9,
    'flippers': 25,
}

# Value that will return false for all relevant checks
ITEM_NEVER_FOUND = 100_000_000

RUN_START = 20_000
RUN_END = 20_001

TILE_MASK = 0
LOCATION_MASK = 1000
EVENT_MASK = 2000

def item_log_idx(df: DataFrame, item: str, progressive_level: int = 1) -> int:
    found_items = df[df.item_id == ITEM_IDS[item]].index
    return found_items[progressive_level-1] if len(found_items) >= progressive_level else ITEM_NEVER_FOUND

def item_log_idx(df: DataFrame, item: str, progressive_level: int = 1) -> int:
    found_items = df[df.item_id == ITEM_IDS[item]].index
    return (
        found_items[progressive_level - 1]
        if len(found_items) >= progressive_level
        else ITEM_NEVER_FOUND
    )

def can_slash(df: DataFrame) -> DataFrame:
    df['can_slash'] = df.index >= item_log_idx(df, 'sword')
    return df

def can_hammer_things(df: DataFrame) -> DataFrame:
    df['can_hammer'] = df.index >= item_log_idx(df, 'hammer')
    return df

def can_dash(df: DataFrame) -> DataFrame:
    df['can_dash'] = df.index >= item_log_idx(df, 'boots')
    return df

def can_shoot(df: DataFrame) -> DataFrame:
    df['can_shoot'] = df.index >= item_log_idx(df, 'bow')
    return df

def can_lift_rocks(df: DataFrame) -> DataFrame:
    df['can_lift_rocks'] = df.index >= item_log_idx(df, 'glove')
    return df

def can_lift_heavy_rocks(df: DataFrame) -> DataFrame:
    df['can_lift_heavy_rocks'] = df.index >= item_log_idx(df, 'glove', 2)
    return df

def can_remain_link_in_dw(df: DataFrame) -> DataFrame:
    df['can_remain_link_in_dw'] = df.index >= item_log_idx(df, 'pearl')
    return df

def can_burn_things(df: DataFrame) -> DataFrame:
    df['can_burn_things'] = df.index >= item_log_idx(df, 'firerod')
    return df

def can_melt_things(df: DataFrame) -> DataFrame:
    can_melt_idx = min(item_log_idx(df, 'firerod'), item_log_idx(df, 'bombos'))
    df['can_melt_things'] = df.index >= can_melt_idx
    return df

def can_light_things(df: DataFrame) -> DataFrame:
    can_light_idx = min(item_log_idx(df, 'firerod'), item_log_idx(df, 'lantern'))
    df['can_light_things'] = df.index >= can_light_idx
    return df

def can_traverse_big_gaps(df: DataFrame) -> DataFrame:
    df['can_traverse_big_gaps'] = df.index >= item_log_idx(df, 'hookshot')
    return df

def can_swim(df: DataFrame) -> DataFrame:
    df['can_swim'] = df.index >= item_log_idx(df, 'flippers')
    return df

def can_pass_energy_barriers(df: DataFrame) -> DataFrame:
    can_pass_idx = min(item_log_idx(df, 'sword', progressive_level=2), item_log_idx(df, 'byrna'), item_log_idx(df, 'cape'))
    df['can_pass_energy_barriers'] = df.index >= can_pass_idx
    return df

def create_row_hash(row):
    key_tuple = tuple((val for _, val in sorted(row.items(), key=lambda kv: kv[0])))
    return hash(key_tuple)

def apply_current_abilities(df: DataFrame) -> DataFrame:
    df = can_slash(df)
#    df = can_hammer_things(df)
    df = can_dash(df)
#    df = can_shoot(df)
    df = can_lift_rocks(df)
#    df = can_lift_heavy_rocks(df)
    df = can_remain_link_in_dw(df)
#    df = can_burn_things(df)
#    df = can_melt_things(df)
#    df = can_light_things(df)
    df = can_traverse_big_gaps(df)
#    df = can_swim(df)
    ability_columns = [c for c in df.columns if c.startswith('can_')]
    df['ability_hash'] = df[ability_columns].apply(create_row_hash, axis=1)
    # df = df.drop(columns=ability_columns)

    return df

def only_movements(df: DataFrame) -> DataFrame:
    df['movement_id'] = np.nan
    df.loc[df.tile_id.notna(), 'movement_id'] = df.loc[df.tile_id.notna(), 'tile_id'] + TILE_MASK
    df.loc[df.location_id.notna(), 'movement_id'] = df.loc[df.location_id.notna(), 'location_id'] + LOCATION_MASK
    df.loc[df.event_id.notna(), 'movement_id'] = df.loc[df.event_id.notna(), 'event_id'] + EVENT_MASK

    return df[df.movement_id.notna()].drop(columns=['item_id', 'event_id', 'location_id', 'tile_id'])

def add_previous_and_next_move(df: DataFrame) -> DataFrame:
    '''Used to create a `route_hash`'''
    df['preprevious_move'] = df.movement_id.shift(2, fill_value=RUN_START)
    df['previous_move'] = df.movement_id.shift(1, fill_value=RUN_START)
    df['next_move'] = df.movement_id.shift(-1, fill_value=RUN_END)
    df['nextnext_move'] = df.movement_id.shift(-2, fill_value=RUN_END)
    return df

def apply_routes(df: DataFrame) -> DataFrame:
    df = add_previous_and_next_move(df)
    move_columns = [c for c in df.columns if 'move' in c]
    df['route_id'] = df[move_columns].apply(create_row_hash, axis=1)
    return df

def add_time_deltas(df: DataFrame) -> DataFrame:
    start_time = df['timestamp'].min()
    df['timestamp'] = df['timestamp'] - start_time
    df['time_delta'] = df['timestamp'] - df['timestamp'].shift(1, fill_value=0)
    return df

def apply_route_ability_hash(df: DataFrame) -> DataFrame:
    if not 'ability_hash' in df.columns:
        df = apply_current_abilities(df)
    if not 'route_id' in df.columns:
        df = only_movements(df)
        df = apply_routes(df)
    df['route_ability_id'] = df[['route_id', 'ability_hash']].apply(create_row_hash, axis=1)
    return df

def convert_legacy_csv(df: DataFrame) -> DataFrame:
    if "transition_id" in df.columns:
        df["tile_id"] = df["transition_id"]
        df = df.drop(columns=["transition_id"])
    return df

def read_runs(glob_path: str):
    dfs: List[DataFrame] = []
    for path in Path.cwd().glob(glob_path):
        meta = read_meta(path)
        df = pd.read_csv(path, skiprows=meta['header'])
        df = convert_legacy_csv(df)
        df = apply_route_ability_hash(df)
        df = add_time_deltas(df)
        df['filename'] = path.name
        dfs.append(df)
    return pd.concat(dfs)

def best_route_time(runs):
    def wrapped(row):
        return runs[runs.route_ability_id == row.route_ability_id].time_delta.min()
    return wrapped

def read_meta(input_file: str) -> dict:
    meta = {'header': 0}
    with open(input_file) as f:
        while True:
            line = f.readline()
            if line[0] in ['#', '\n']:
                meta['header'] += 1
                if line[0] == '#':
                    line = line.strip().split(' ')
                    meta[line[1]] = ' '.join(line[2:])
            else:
                break
    return meta

class RunComparator:
    def __init__(self, all_runs: DataFrame) -> None:
        self.all_runs: DataFrame = all_runs
        self.best_route_time = best_route_time(self.all_runs)

    def apply_best_route_time(self, selected_run):
        rows = []
        for _, row in selected_run.iterrows():
            row["best_time_delta"] = self.best_route_time(row)
            row['deltadelta'] = row['time_delta'] - row['best_time_delta']
            rows.append(row)
        return DataFrame.from_records(rows)

    def translate_ids(self, selected_run):
        rows = []
        move_col_names = ['-2', '-1', '0', '+1', '+2']
        for _, row in selected_run.iterrows():
            moves = []
            move_ids = [row['preprevious_move'], row['previous_move'], row['movement_id'], row['next_move'], row['nextnext_move']]
            for move_id in move_ids:
                if move_id >= 2000:
                    moves.append(EVENTS.loc[EVENTS.id == move_id - 2000, 'name'])
                elif move_id >= 1000:
                    moves.append(CHECKS.loc[CHECKS.id == move_id - 1000, 'name'])
                else:
                    moves.append(TILES.loc[TILES.id == move_id - 0, 'name'])
            for move,name in zip(moves, move_col_names):
                row[name] = move
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
            processed = self.translate_ids(processed)
            print("Run: ", filename)
            print(
                f"Total time: {datetime.fromtimestamp(processed.time_delta.sum()/ 1000) - datetime.fromtimestamp(0)}"
            )
            print(
                f"Best possible time: {datetime.fromtimestamp(processed.best_time_delta.sum()/1000) - datetime.fromtimestamp(0)}\n"
            )
            return processed

def main():
    compare = RunComparator(read_runs('RUNS/*.csv'))
    input_file = 'Lightspeed - 20211219_135644.csv'
    run = compare.best_possible_time_for(input_file)
    run.to_excel(input_file.replace('.csv', '.xlsx'))

if __name__ == '__main__':
    main()