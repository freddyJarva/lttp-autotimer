<script>
	/// <reference path="../typedefs.js" />
	import '$lib/styles.css';

	import { onMount } from 'svelte';
	import { listen } from '@tauri-apps/api/event';
	import { invoke } from '@tauri-apps/api/tauri';
	import {
		fmtDelta,
		fmtObjective,
		runTime,
		fmTime,
		tiles,
		getBestRun,
		sum,
		isDuplicateEvent,
		isCombinedEvent
	} from '$lib/util';

	let unlisten_snes_events;

	let isRecording = false;

	let current_tile_idx = -1;
	$: currentTile = tiles[current_tile_idx] ?? null;

	let /** @type {RunObjectives} */ runObjectives = {
			start_tile: current_tile_idx,
			objectives: [],
			finalized: false
		};
	$: lastObjective = runObjectives.objectives.length ? runObjectives.objectives[runObjectives.objectives.length - 1] : null;
	let currentObjective = 0;
	let /** @type {SnesEvent[]} */ currentRun = [];
    $: bestRun = getBestRun(times) ?? [];
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
		updateTimes(currentRun);
		runStarted = false;
		runFinished = true;
	}

	/**
	 * update the times array with the new run
	 *
	 * @param {SnesEvent[]} run
	 */
	function updateTimes(run) {
		for (let i = 1; i < run.length; i++) {
			if (times[i - 1] === undefined) {
				times[i - 1] = [run[i].timestamp - run[i - 1].timestamp];
			} else {
				let segmentTimes = times[i - 1];
				times[i - 1] = [...segmentTimes, run[i].timestamp - run[i - 1].timestamp];
			}
		}
		times = times;
		bestRun = getBestRun(times);
	}

	async function startRecording() {
		runObjectives = {
			start_tile: current_tile_idx,
			objectives: [],
			finalized: false
		};
		bestRun = [];
		currentRun = [];
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
		updateTimes(runObjectives.objectives);
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
			if (snesEvent.tile_id !== null) {
				current_tile_idx = snesEvent.tile_id;
			}
			if (isRecording) {
				// TODO: combine objectives instead of ignoring if they occur at the same time
				if (
					isDuplicateEvent(lastObjective, snesEvent) ||
					isCombinedEvent(lastObjective, snesEvent)
				) {
					return;
				}
				runObjectives.objectives = [...runObjectives.objectives, snesEvent];
			}
		});
	});
</script>

<h1>A Link to The Past Autotimer</h1>

<div class="page-container" style="display:grid; grid-template-columns: auto auto;">
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

		<div class="run-column">
			<h3>Objectives</h3>
			{#if runObjectives.objectives.length > 1}
				<ul>
					{#each runObjectives.objectives.slice(1) as o}
						<li>{fmtObjective(o)}</li>
					{/each}
				</ul>
			{/if}
		</div>

		<div class="run-column">
			{#if runStarted || runFinished}
				<h3>Current Run</h3>
				<ul>
					{#each currentRun.slice(1) as cleared, idx}
						<li>{fmtDelta(cleared.timestamp, currentRun[idx]?.timestamp)}</li>
					{/each}
					{#if runFinished}
						<li style="border-top: 1px solid #e7e7e7; margin-top: 10px;"></li>
						<li class="run-total">{fmTime(runTime(currentRun) ?? 0)}</li>
					{/if}
				</ul>
			{:else if runObjectives.objectives.length > 1}
				<h3>Initial run</h3>
				<ul>
					{#each runObjectives.objectives.slice(1) as cleared, idx}
						<li>{fmtDelta(cleared.timestamp, runObjectives.objectives[idx]?.timestamp)}</li>
					{/each}
					{#if runObjectives.finalized}
						<li style="border-top: 1px solid #e7e7e7; margin-top: 10px;"></li>
						<li class="run-total">{fmTime(runTime(runObjectives.objectives) ?? 0)}</li>
					{/if}
				</ul>
			{:else}
				<h3>No run recorded</h3>
			{/if}
		</div>

		<div class="run-column">
			<h3>Best</h3>
			<ul>
				{#each bestRun as segmentTime}
					<li>{fmTime(segmentTime)}</li>
				{/each}
				{#if times.length > 0}
					<li style="border-top: 1px solid #e7e7e7; margin-top: 10px;"></li>
					<li class="run-total">{fmTime(sum(...bestRun))}</li>
				{/if}
			</ul>
		</div>
	</div>

	<footer style="grid-column: 1 / span 3; grid-row: 6;">
		{#if currentTile}
			{#if currentTile.indoors}
				<p>{currentTile.name}</p>
			{:else}
				<p>{currentTile.region} - {currentTile.name}</p>
			{/if}
		{/if}
	</footer>
</div>

<style>
	ul {
		list-style: none;
		padding: 0;
	}

	.run-times {
		grid-column: 1 / span 3;
		grid-row: 2;
		display: grid;
		grid-template-columns: auto auto;
		font-size: 1.8em;
	}

	.page-container {
		display: grid;
		grid-template-columns: 1fr auto;
		min-height: 100vh;
	}

	.run-total {
		color: rgb(255, 255, 255);
		text-align: left;
	}

	.run-column {
		grid-column: auto;
		grid-row: span 3;
		/* border: 1px solid #e7e7e7; */
		padding: 10px;
		border-radius: 5px;
		margin: 10px;
	}

	/* Footer will be like a console output line in the bottom, occlude things that go into it */
	footer {
		text-align: center;
		border-top: 1px solid #e7e7e7;
		font-size: 1.8em;
		background-color: #0f0f0f;
	}
</style>
