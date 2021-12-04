# ALttPR Autotimer

## Adding new triggers
If there's content missing (i.e. not being written out in the app or to the csv file) that you'd like to add, here are some tips to find the correct values. I'll be using snes9x for finding values, but feel free to use any other emulator/tool that allows you to read the snes.

### Terminology
- <b>Tile</b>: For a given area, everywhere you can go without triggering a transition is considered to be the same tile. I do use this term incorrectly however since there can actually exist multiple tiles inside such an area, but for this application we do not care about that (yet)
- <b>Overworld</b> - all tiles that are outside. For a more technical definition, all tiles where address `7e001b` is `0`
- <b>Underworld</b> - All tiles that are inside. For a more technical definition, all tiles where address `7e001b` is `1` 

### Ram watch
Here follows an example of watching the snes ram address keeping track of the current overworld tile.

Start snes9x and load a lttp rando rom.

Go to Cheat -> Ram Watch

A new window should appear titled `Ram Watch`

Go to Watches -> New Watch

In the window `Edit Watch`, type in the following:
- Address: `7e040a`
- Notes: `Overworld tile` (or something else that helps you remember what this address is for)
- Data Type: `Hexadecimal`
- Data Size: `1 byte`

Click OK. The address should be added to the `Ram Watch` window.

If you go to a new screen in the overworld, you should see the value for that address change.

### Transitions

#### Overworld
All overworld transitions should be mapped already, but if there for some reason is a missing tile or a tile with an incorrect value, it can be found by adding a ram watch for the values given in the above example, and then going to that tile.

Let's say there was no transition triggered for Overworld Hyrule Castle and you'd like to add it: 
- Begin by walking to Hyrule Castle and see the value in your ram watch change to `1b` (or `27` if you didn't set the Data Type to `Hexadecimal`). 
- Next, open up `src/transitions.json`, and add the following: 
```json 
{
    "name": "Hyrule Castle",
    "indoors": false,
    "address_value": [
        "0x1B"
    ]
}
```
- That's it!

#### Underworld