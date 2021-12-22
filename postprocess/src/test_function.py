from .function import LogicTile, create_row_hash


def test_next_logical_tile_generates_correct_id():
    # Given
    logic_tile = LogicTile(
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
    assert logic_tile.next_logical_tile == create_row_hash(
        row_2, cols=["previous_tile", "tile_id", "next_tile", "can_dash", "can_swim"]
    )


def test_logic_tile_1s_next_logic_tile_should_equal_logic_tile_2s_id():
    logic_tile_1 = LogicTile(
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
    logic_tile_2 = LogicTile(
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
