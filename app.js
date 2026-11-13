import init, { Controller, Interval } from './pkg/traffic_lights.js';

// min/max/pass mirror Controller::demo()'s defaults in src/lib.rs, modeled
// after a Siemens M60 timing chart: main street > side street > left turns.
const phases = [
  { id: 1, label: 'West left', path: 'M200 342H264C289 342 342 289 342 264V200', x: 196, y: 342, labelX: 166, labelY: 348, min: 5, max: 20, pass: 3 },
  { id: 2, label: 'East through', path: 'M440 254H200', x: 444, y: 254, labelX: 474, labelY: 260, min: 15, max: 50, pass: 4 },
  { id: 3, label: 'North left', path: 'M298 200V264C298 289 351 342 376 342H440', x: 298, y: 196, labelX: 298, labelY: 168, min: 5, max: 15, pass: 3 },
  { id: 4, label: 'South through', path: 'M386 440V200', x: 386, y: 444, labelX: 386, labelY: 483, min: 10, max: 35, pass: 3 },
  { id: 5, label: 'East left', path: 'M440 298H376C351 298 298 351 298 376V440', x: 444, y: 298, labelX: 474, labelY: 304, min: 5, max: 20, pass: 3 },
  { id: 6, label: 'West through', path: 'M200 386H440', x: 196, y: 386, labelX: 166, labelY: 392, min: 15, max: 50, pass: 4 },
  { id: 7, label: 'South left', path: 'M342 440V376C342 351 289 298 264 298H200', x: 342, y: 444, labelX: 342, labelY: 483, min: 5, max: 15, pass: 3 },
  { id: 8, label: 'North through', path: 'M254 200V440', x: 254, y: 196, labelX: 254, labelY: 168, min: 10, max: 35, pass: 3 },
];

const byId = (id) => document.getElementById(id);
let controller;
let timer = null;

// Relative arrival likelihood per simulated second: through movements busier
// than their paired left turn, and the main street (1/2/5/6) only slightly
// busier than the side street (3/4/7/8) within each movement type.
const ARRIVAL_CHANCE = { 1: .13, 2: .27, 3: .11, 4: .22, 5: .13, 6: .27, 7: .11, 8: .22 };
const QUEUE_CAP = 6;
const TRANSIT_MS = 850;

// Each car is { id, phase, state: 'queued' | 'moving', moveStart? }.
// Queued cars sit stacked behind the stop line; moving cars animate along
// the phase's own movement path in real time, independent of sim speed.
let nextCarId = 1;
const cars = [];

function spawnRandomTraffic() {
  if (!byId('random-traffic').checked) return;
  for (const phase of phases) {
	// The detector fires on the roll alone, same as a real vehicle arriving;
	// QUEUE_CAP only limits how many waiting cars get drawn, never demand.
	if (Math.random() < ARRIVAL_CHANCE[phase.id]) {
	  controller.call(phase.id);
	  const queued = cars.filter((car) => car.phase === phase.id && car.state === 'queued').length;
	  if (queued < QUEUE_CAP) cars.push({ id: nextCarId++, phase: phase.id, state: 'queued' });
	}
  }
}

function dischargeTraffic() {
  for (const phase of phases) {
	if (controller.signal(phase.id) !== Interval.Green) continue;
	const front = cars.find((car) => car.phase === phase.id && car.state === 'queued');
	if (front) {
	  front.state = 'moving';
	  front.moveStart = performance.now();
	}
  }
}

function resetTraffic() {
  cars.length = 0;
}

function advance() {
  controller.step();
  spawnRandomTraffic();
  dischargeTraffic();
  render();
}

// Direction of travel (degrees) at a given length along a movement path.
function pathAngle(path, length, total) {
  const a = path.getPointAtLength(Math.max(0, length - 1));
  const b = path.getPointAtLength(Math.min(total, length + 1));
  return Math.atan2(b.y - a.y, b.x - a.x) * 180 / Math.PI;
}

// Stacked waiting slot upstream of the stop line, facing into the intersection.
function queueSlot(path, total, index) {
  const start = path.getPointAtLength(0);
  const ahead = path.getPointAtLength(Math.min(8, total));
  const dx = ahead.x - start.x;
  const dy = ahead.y - start.y;
  const length = Math.hypot(dx, dy) || 1;
  const ux = dx / length;
  const uy = dy / length;
  return {
	x: start.x - ux * (index + 1) * 23,
	y: start.y - uy * (index + 1) * 23,
	angle: Math.atan2(dy, dx) * 180 / Math.PI,
  };
}

// Runs continuously so moving cars animate smoothly regardless of sim speed.
function renderCars() {
  for (let i = cars.length - 1; i >= 0; i--) {
	const car = cars[i];
	if (car.state === 'moving' && performance.now() - car.moveStart >= TRANSIT_MS) cars.splice(i, 1);
  }

  const queuedSoFar = {};
  const rects = cars.map((car) => {
	const path = byId(`movement-${car.phase}`);
	const total = path.getTotalLength();
	let x, y, angle;

	if (car.state === 'moving') {
	  const progress = Math.min(1, (performance.now() - car.moveStart) / TRANSIT_MS);
	  const length = progress * total;
	  ({ x, y } = path.getPointAtLength(length));
	  angle = pathAngle(path, length, total);
	} else {
	  const index = queuedSoFar[car.phase] ?? 0;
	  queuedSoFar[car.phase] = index + 1;
	  ({ x, y, angle } = queueSlot(path, total, index));
	}

	return `<rect x="${(x - 10).toFixed(1)}" y="${(y - 5.5).toFixed(1)}" width="20" height="11" rx="2.5" transform="rotate(${angle.toFixed(1)} ${x.toFixed(1)} ${y.toFixed(1)})"/>`;
  });
  byId('cars').innerHTML = rects.join('');

  requestAnimationFrame(renderCars);
}

function buildControls() {
  byId('phase-controls').innerHTML = phases.map((phase) => `
	<tr>
	  <th scope="row">${phase.id}</th>
	  <td>${phase.label}</td>
	  <td>
		<span id="signal-${phase.id}" class="signal-word signal-red">RED</span>
		<span id="timer-${phase.id}" class="phase-timer"></span>
	  </td>
	  <td><button class="btn btn-outline-primary btn-sm call-button" data-call="${phase.id}" type="button" aria-label="Detector call for phase ${phase.id}" disabled>Call</button></td>
	  <td><div class="form-check form-switch"><input class="form-check-input" type="checkbox" role="switch" id="recall-${phase.id}" aria-label="Minimum recall for phase ${phase.id}" ${phase.id === 2 || phase.id === 6 ? 'checked' : ''} disabled></div></td>
	</tr>
  `).join('');

  byId('phase-timing').innerHTML = phases.map((phase) => `
	<tr>
	  <th scope="row">${phase.id}</th><td>${phase.label}</td>
	  <td><input class="form-control form-control-sm" id="min-${phase.id}" aria-label="Phase ${phase.id} minimum green seconds" type="number" min="1" max="255" step="1" value="${phase.min}" required></td>
	  <td><input class="form-control form-control-sm" id="max-${phase.id}" aria-label="Phase ${phase.id} maximum green seconds" type="number" min="1" max="255" step="1" value="${phase.max}" required></td>
	  <td><input class="form-control form-control-sm" id="pass-${phase.id}" aria-label="Phase ${phase.id} passage seconds" type="number" min="1" max="255" step="1" value="${phase.pass}" required></td>
	</tr>
  `).join('');

  byId('movements').innerHTML = phases.map((phase) => `<path id="movement-${phase.id}" class="movement" d="${phase.path}" stroke="#899096" opacity=".25" marker-end="url(#arrow-off)"/>`).join('');
  byId('lights').innerHTML = phases.map((phase) => `
	<g id="light-${phase.id}" role="img" aria-label="Phase ${phase.id}: red">
	  <rect class="signal-box" x="${phase.x - 20}" y="${phase.y - 20}" width="40" height="40"/>
	  <circle id="lamp-${phase.id}" cx="${phase.x}" cy="${phase.y}" r="15" fill="#dc3545"/>
	  <text id="lamp-letter-${phase.id}" class="lamp-letter" x="${phase.x}" y="${phase.y}" fill="white">R</text>
	  <text class="phase-number" x="${phase.labelX}" y="${phase.labelY}">${phase.id}</text>
	</g>
  `).join('');
}

function render() {
  const green = [];
  const yellow = [];
  let clearing = false;
  const pending = controller.pending_calls();

  for (const phase of phases) {
	const signal = controller.signal(phase.id);
	const isGreen = signal === Interval.Green;
	const isYellow = signal === Interval.Yellow;
	const name = isGreen ? 'green' : isYellow ? 'yellow' : 'red';
	const color = isGreen ? '#35c66a' : isYellow ? '#ffc107' : '#dc3545';

	if (isGreen) green.push(phase.id);
	if (isYellow) yellow.push(phase.id);
	if (signal === Interval.RedClearance) clearing = true;

	byId(`lamp-${phase.id}`).setAttribute('fill', color);
	byId(`lamp-letter-${phase.id}`).textContent = name[0].toUpperCase();
	byId(`lamp-letter-${phase.id}`).setAttribute('fill', isYellow ? '#212529' : 'white');
	byId(`light-${phase.id}`).setAttribute('aria-label', `Phase ${phase.id}: ${name}`);
	const path = byId(`movement-${phase.id}`);
	path.setAttribute('stroke', isGreen || isYellow ? color : '#899096');
	path.setAttribute('opacity', isGreen || isYellow ? '1' : '.18');
	path.setAttribute('marker-end', `url(#arrow-${isGreen ? 'green' : isYellow ? 'yellow' : 'off'})`);

	const word = byId(`signal-${phase.id}`);
	word.className = `signal-word signal-${name}`;
	word.textContent = name.toUpperCase();
	const elapsed = controller.ring_elapsed(phase.id);
	const timer = byId(`timer-${phase.id}`);
	if (elapsed === undefined) {
	  timer.textContent = '';
	} else {
	  const gap = controller.ring_gap(phase.id);
	  timer.textContent = gap > 0 ? `${elapsed}s (+${gap}s)` : `${elapsed}s`;
	}
	const call = document.querySelector(`[data-call="${phase.id}"]`);
	const waiting = (pending & (1 << (phase.id - 1))) !== 0;
	call.classList.toggle('waiting', waiting);
	call.textContent = waiting ? 'Waiting' : 'Call';
  }

  const seconds = controller.time();
  byId('clock').textContent = `${String(Math.floor(seconds / 60)).padStart(2, '0')}:${String(seconds % 60).padStart(2, '0')}`;
  const screenView = document.querySelector('input[name="screen-view"]:checked').value;
  byId('controller-screen').textContent = screenView === 'configured' ? controller.render_timing() : controller.render();
  byId('signal-summary').textContent = green.length ? `Green: ${green.join(' / ')}`
	: yellow.length ? `Yellow: ${yellow.join(' / ')}`
	: clearing ? 'Red clearance Â· all red' : 'Startup Â· all red';
}

function pause() {
  clearInterval(timer);
  timer = null;
  byId('run').textContent = 'Run';
  byId('run').className = 'btn btn-primary';
  byId('run').setAttribute('aria-pressed', 'false');
  byId('run-state').textContent = 'Paused';
  byId('step').disabled = false;
}

function run() {
  const delay = 1000 / Number(byId('speed').value);
  timer = setInterval(advance, delay);
  byId('run').textContent = 'Pause';
  byId('run').className = 'btn btn-warning';
  byId('run').setAttribute('aria-pressed', 'true');
  byId('run-state').textContent = 'Running';
  byId('step').disabled = true;
}

function updateRecall() {
  let mask = 0;
  for (const phase of phases) {
	if (byId(`recall-${phase.id}`).checked) mask |= 1 << (phase.id - 1);
  }
  controller.set_min_recall(mask);
  byId('last-action').textContent = `Minimum recall updated. Mask: ${mask}.`;
  render();
}

function applyTiming(event) {
  event.preventDefault();
  if (!byId('timing-form').reportValidity()) return;

  const values = phases.map((phase) => ({
	id: phase.id,
	min: Number(byId(`min-${phase.id}`).value),
	max: Number(byId(`max-${phase.id}`).value),
	pass: Number(byId(`pass-${phase.id}`).value),
  }));
  const yellow = Number(byId('yellow').value);
  const red = Number(byId('red-clearance').value);

  for (const phase of values) {
	if (phase.min > phase.max) {
	  byId('config-error').textContent = `Phase ${phase.id}: maximum green must be at least minimum green.`;
	  byId(`max-${phase.id}`).focus();
	  return;
	}
  }

  pause();
  for (const phase of values) {
	controller.configure_phase(phase.id, phase.min, phase.max, phase.pass, yellow, red);
  }
  controller.reset();
  resetTraffic();
  byId('config-error').textContent = '';
  byId('last-action').textContent = 'Timing applied. Controller reset to startup all red.';
  render();
}

buildControls();
requestAnimationFrame(renderCars);

try {
  await init();
  controller = Controller.demo();
  for (const button of document.querySelectorAll('button, #speed, #random-traffic, [id^="recall-"]')) button.disabled = false;
  pause();
  render();
  byId('last-action').textContent = 'Minimum recall is on for phases 2 and 6 (main street through); every other phase rests until called.';

  byId('run').addEventListener('click', () => timer === null ? run() : pause());
  byId('step').addEventListener('click', advance);
  byId('reset').addEventListener('click', () => {
	pause();
	controller.reset();
	resetTraffic();
	byId('last-action').textContent = 'Controller reset. Applied timing and recall settings retained.';
	render();
  });
  byId('speed').addEventListener('change', () => { if (timer !== null) { pause(); run(); } });
  byId('lcd-toggle').addEventListener('change', () => {
	byId('controller-screen').classList.toggle('lcd-mode', byId('lcd-toggle').checked);
  });
  for (const el of document.querySelectorAll('input[name="screen-view"]')) el.addEventListener('change', render);
  for (const phase of phases) {
	document.querySelector(`[data-call="${phase.id}"]`).addEventListener('click', () => {
	  controller.call(phase.id);
	  byId('last-action').textContent = `Detector hit sent to phase ${phase.id} (${phase.label.toLowerCase()}).`;
	  render();
	});
	byId(`recall-${phase.id}`).addEventListener('change', updateRecall);
  }
  byId('timing-form').addEventListener('submit', applyTiming);
} catch (error) {
  byId('error').classList.remove('d-none');
  byId('error').textContent = `The WASM controller could not load. ${error.message}`;
  byId('run-state').textContent = 'Controller unavailable';
  console.error(error);
}
