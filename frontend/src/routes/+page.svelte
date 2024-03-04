<script>
	/// <reference path="../typedefs.js" />
	import '$lib/styles.css';

	import { onMount } from 'svelte';
	import { listen } from '@tauri-apps/api/event';
	import { invoke } from '@tauri-apps/api/tauri';
	import { fmtDelta, fmtObjective, runTime, fmTime, tiles, eventInfo } from '$lib/util';

	let unlisten_snes_events;

	let isRecording = false;

	let /** @type {SnesEvent[]} */ snesEvents = [];

	let current_tile_idx = -1;
    $: currentTile = tiles[current_tile_idx] ?? null;
	let /** @type {RunObjectives} */ runObjectives = {
			start_tile: current_tile_idx,
			objectives: [],
			finalized: false
		};
	let currentObjective = 0;
	let /** @type {SnesEvent[]} */ currentRun = [];
    let /** @type {number[][]} */ times = [];

	let runStarted = false;
	let runFinished = false;

	/** @param {string} cmd */
	async function sendTimerCommand(cmd) {
		invoke('timer_command', { message: cmd });
	}

	/**
	 * @param {SnesEvent} o
	 * @param {SnesEvent} snesEvent
	 */
	function objectiveCleared(o, snesEvent) {
		if (o === undefined) {
			console.log('objective sent to objectiveCleard is undefined');
			console.log(runObjectives.objectives);
			return false;
		}
		return (
			o.tile_id === snesEvent.tile_id &&
			o.item_id === snesEvent.item_id &&
			o.event_id === snesEvent.event_id &&
			o.location_id === snesEvent.location_id
		);
	}

	/**
	 * Resets all triggered events on rust side.
	 */
	function untriggerEvents() {
		sendTimerCommand('clear_event_log');
	}

	/**
	 * start run setting the given event as the start objective
	 *
	 * @param {SnesEvent} e - The event that triggered the run
	 */
	function startRun(e) {
		console.log('STARTING THE RUN OOO');
		untriggerEvents();
		currentRun = [e];
		currentObjective = 1;
		runStarted = true;
		runFinished = false;
	}

	/**
	 * Increment the run by one
	 *
	 * @param {SnesEvent} e - the objective that was just finished
	 */
	function progressRun(e) {
		currentRun = [...currentRun, e];
		currentObjective++;
		if (currentObjective >= runObjectives.objectives.length) {
			finishRun();
		}
	}

	function abortRun() {
		runStarted = false;
	}

	function finishRun() {
        let /** @type {number[][]} */ newTimes = []
        console.log("HERE WE AT BABY")
        if (times.length > 0) {
            newTimes = [...times]
        }
        for (let i = 1; i < currentRun.length; i++) {
            if (newTimes[i-1] === undefined) {
                console.log("HELL YEAH")
                newTimes[i-1] = [currentRun[i].timestamp - currentRun[i-1].timestamp]
            } else {
                console.log("HELL NO")
                let segmentTimes = newTimes[i-1];
                newTimes[i-1] = [...segmentTimes, currentRun[i].timestamp - currentRun[i-1].timestamp]
            }
        }
        console.log("NEW TIMES", newTimes)
        times = newTimes;
		runStarted = false;
		runFinished = true;
	}

	async function startRecording() {
		runObjectives = {
			start_tile: current_tile_idx,
			objectives: [],
			finalized: false
		};
		untriggerEvents();
		// let rust backend reset event log
		// and read in previous events again before recording
		await new Promise((r) => setTimeout(r, 100));
        runFinished = false;
        runStarted = false;
		isRecording = true;
	}

	function stopRecording() {
		isRecording = false;
		runObjectives.finalized = true;
	}

	onMount(async () => {
		untriggerEvents();
		unlisten_snes_events = await listen('snes_event', (event) => {
			let /** @type {SnesEvent} */ snesEvent = JSON.parse(event.payload);

			if (runStarted) {
				let objective = runObjectives.objectives[currentObjective];
				if (objectiveCleared(objective, snesEvent)) {
					progressRun(snesEvent);
				}
			} else if (
				runObjectives.finalized &&
				runObjectives.start_tile === current_tile_idx &&
				objectiveCleared(runObjectives.objectives[0], snesEvent)
			) {
				startRun(snesEvent);
			} else if (runObjectives.finalized && snesEvent.tile_id === runObjectives.start_tile) {
                untriggerEvents();
            }
			if (snesEvent.tile_id) {
				current_tile_idx = snesEvent.tile_id;
			}
			snesEvents = [...snesEvents, event.payload];
			if (isRecording) {
				runObjectives.objectives = [...runObjectives.objectives, snesEvent];
				if (snesEvent.tile_id) {
				}
			}
		});
	});
</script>

<h1>A Link to The Past Autotimer</h1>

<div style="display:grid; grid-template-columns: auto auto;">
	<div style="grid-column: 1; grid-row: 1;">
		{#if isRecording}
			<button on:click={stopRecording}>Stop Recording</button>
		{:else}
			<button on:click={startRecording}>Start Recording</button>
		{/if}
	</div>

	<div style="grid-column: 2; grid-row: 1;">
		<button disabled={!runStarted} on:click={abortRun}>Abort run</button>
	</div>

    <div class="run-times">

        <div style="grid-column: span 3 ; grid-row: 1;">
            {#if runObjectives.objectives.length > 0}
                <p>Start Location: {tiles[runObjectives.start_tile].name}</p>
                <p>Trigger: {fmtObjective(runObjectives.objectives[0])}</p>
            {:else}
                <p>Start Location: not recorded</p>
            {/if}
        </div>

        <div style="grid-column: 1; grid-row: 2;">
            <h3>Objectives</h3>
            {#if runObjectives.objectives.length > 1}
                <ol>
                    {#each runObjectives.objectives.slice(1) as o}
                        <li>{fmtObjective(o)}</li>
                    {/each}
                </ol>
            {:else}
                <ol>
                    <li>Eat a berry</li>
                    <li>Sing a song</li>
                    <li>Fuck the police</li>
                </ol>
            {/if}
        </div>

        <div style="grid-column: 2; grid-row: 2;">
            {#if runStarted}
                <h3>Current Run</h3>
                <ul>
                    {#each currentRun.slice(1) as cleared, idx}
                        <li>{fmtDelta(cleared.timestamp, currentRun[idx]?.timestamp)}</li>
                    {/each}
                </ul>
            {:else if runFinished}
                <h3>Finished {fmTime(runTime(currentRun) ?? 0)}</h3>
                <ul>
                    {#each currentRun.slice(1) as cleared, idx}
                        <li>{fmtDelta(cleared.timestamp, currentRun[idx]?.timestamp)}</li>
                    {/each}
                </ul>
            {:else}
                <h3>No Active Run</h3>
                <ul>
                    <li>0.00</li>
                    <li>13.24</li>
                    <li>2.20</li>
                </ul>
            {/if}
        </div>

        <div style="grid-column: 3; grid-row: 2;">
            <h3>Best times</h3>
            <ul>
                {#each times as segmentTimes}
                    <li>{fmTime(Math.min(...segmentTimes))}</li>
                {/each}
            </ul>
        </div>
    </div>

    <footer style="grid-column: 1 / span 3; grid-row: 4;">
		{#if currentTile}
            <p>{currentTile.region} - {currentTile.name}</p>
		{/if}
    </footer>

</div>

<style>
	ul {
		list-style: none;
	}

	.run-times {
        grid-column: 1 / span 3;
        grid-row: 2;
        display: grid;
        grid-template-columns: auto auto;
        font-size: 1.8em;
	}

    /* Footer will be like a console output line in the bottom */    
    footer {
        text-align: center;
        position: fixed;
        bottom: 0;
        width: 100%;
        padding: 2px;
        border-top: 1px solid #e7e7e7;
        font-size: 1.8em;
    }
</style>
