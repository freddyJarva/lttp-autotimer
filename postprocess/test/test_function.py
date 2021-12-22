from postprocess import function
import pandas as pd


def test_next_logical_tile_generates_correct_id():
    # Given
    logic_tile = function.LogicTile(
        {
            "previous_tile": 4.0,
            "tile_id": 5.0,
            "next_tile": 10.0,
            "next_next_tile": 12.0,
            "can_dash": False,
            "can_swim": False,
        }
    )

    row_2 = {
        "previous_tile": 5.0,
        "tile_id": 10.0,
        "next_tile": 12.0,
        "next_next_tile": 18.0,
        "can_dash": False,
        "can_swim": False,
    }
    # When / Then)
    assert logic_tile.next_logical_tile == function.create_row_hash(
        row_2, cols=["previous_tile", "tile_id", "next_tile", "can_dash", "can_swim"]
    )


def test_logic_tile_1s_next_logic_tile_should_equal_logic_tile_2s_id():
    logic_tile_1 = function.LogicTile(
        {
            "timestamp": 509660,
            "tile_id": 6.0,
            "location_id": None,
            "item_id": None,
            "event_id": None,
            "filename": "20211216_100602.csv",
            "can_slash": True,
            "can_dash": False,
            "can_lift_heavy_rocks": False,
            "can_remain_link_in_dw": False,
            "can_traverse_big_gaps": False,
            "prevprevious_tile": 8.0,
            "previous_tile": 7.0,
            "next_tile": 3.0,
            "next_next_tile": 51.0,
            "time_delta": 3213,
        }
    )
    logic_tile_2 = function.LogicTile(
        _row={
            "timestamp": 512873,
            "tile_id": 3.0,
            "location_id": None,
            "item_id": None,
            "event_id": None,
            "filename": "20211216_100602.csv",
            "can_slash": True,
            "can_dash": False,
            "can_lift_heavy_rocks": False,
            "can_remain_link_in_dw": False,
            "can_traverse_big_gaps": False,
            "prevprevious_tile": 7.0,
            "previous_tile": 6.0,
            "next_tile": 51.0,
            "next_next_tile": 3.0,
            "time_delta": 22236,
        },
    )
    assert logic_tile_1.next_logical_tile == logic_tile_2.id


import csv


def test_GIVEN_row_with_location_id_WHEN_add_check_ocurred_on_tile_THEN_set_check_ocurred_TRUE_on_previous_row_with_a_tile_id(
    tmp_path,
):
    # Given
    csv_data = """\
timestamp,tile_id,location_id,item_id,event_id
0,20,,,
1,,18,,
2,17,,,
3,,,20,
4,30,,,
5,,,,20
6,31,,,
7,,,,20
8,,,20,
9,,55,,
9,55,,,
"""
    df_path = tmp_path / "test.csv"
    # Let pandas read from file so it more closely matches code functionality
    with open(df_path, "w") as f:
        writer = csv.writer(f)
        for row in csv_data.split("\n"):
            writer.writerow(row.split(","))
    df = pd.read_csv(df_path)
    # When
    actual = next(function.add_check_ocurred_on_tile([df]))
    # Then
    expected = pd.read_csv(df_path)
    expected["check_ocurred"] = 0
    tile_rows_checks_ocurred_on = [0, 6]
    for idx in range(len(df)):
        if idx in tile_rows_checks_ocurred_on:
            expected.loc[idx, "check_ocurred"] = 1
        else:
            expected.loc[idx, "check_ocurred"] = 0
    print("ACTUAL:\n", actual)
    print("EXPECTED:\n", expected)

    # assertions does not find nan == nan to be true
    df.fillna(-1, inplace=True)
    expected.fillna(-1, inplace=True)
    for (actual_item, expected_item) in zip(
        actual.to_dict("records"), expected.to_dict("records")
    ):
        assert actual_item == expected_item
