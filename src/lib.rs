use std::fmt;
use wasm_bindgen::prelude::*;

/*
 * Descriptive comments/inspuration sourced from here:
 * https://www.cedengineering.com/userfiles/Introduction%20to%20Traffic%20Signal%20Phasing-R1.pdf
 *
 * This first version uses four fixed pairs:
 * ring 1: 1 -> 2 | 3 -> 4
 * ring 2: 5 -> 6 | 7 -> 8
 * The vertical line marks the main/side street barrier.
 * Each pair completes green, yellow and red clearance together.
 */

/*
 * NEMA (the National Electrical Manufacturers Association) follows a generally
 * accepted procedure for numbering the eight standard vehicle phases of an intersection,
 * a procedure commonly referred to as NEMA signal phasing. When NEMA vehicle
 * phasing is in use, odd numbers are used to designate the left turn phases while even
 * numbers are used to designate the thru phases. A maximum of four left turn phases
 * and four thru phases can be designated using NEMA phasing:

 * Numbers 1 and 5 are used to designate the main street left turn phases while numbers
 * 2 and 6 are used to designate the main street thru phases. Phases 1 and 6 are located
 * on the same approach while phases 2 and 5 are located on the opposite approach.
 * Numbers 3 and 7 are used to designate the cross street left turn phases while numbers
 * 4 and 8 are used to designate the cross street thru phases. Phases 3 and 8 are located
 * on the same approach while phases 4 and 7 are located on the opposite approach.
*/
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhaseId(u8);

impl PhaseId {
    pub fn new(number: u8) -> Option<Self> {
        if !(1..=8).contains(&number) {
            return None;
        }

        Some(Self(number))
    }

    // convert num 1-8 to a bit representation for the timing tables
    fn bit(self) -> u8 {
        1u8 << (self.0 - 1)
    }
}

/*
 * A phase is any traffic signal display with its
 * own set of timings that controls an individual vehicle or pedestrian movement.
 */
#[derive(Clone)]
pub struct Phase {
    id: PhaseId,
    min_seconds: u8,
    max_seconds: u8,
    pass_seconds: u8,
    yellow_seconds: u8,
    red_seconds: u8,
    /*
     * A vehicle phase is serviced when it becomes active and displays a green (G) indication.
     * A vehicle phase remains active thru its subsequent yellow (Y) interval and all-red
     * interval (R).
     */

    /*
     * A pedestrian phase is serviced when it becomes active and displays a
     * walk (W) indication.
     * A pedestrian phase remains active thru its subsequent flashing
     * don’t walk (FDW) interval, which is also referred to as the pedestrian clearance
     * interval.
     */

    /* PHASE SERVICE
     *                | PED 2 active |
     *    |__G__|_Y|AR|_G&W|__G&FDW__|_Y|_AR|
     *    | 01 active |      02 active      |
     */
    // Service is tracked by RingState::interval in this vehicle-only version.
}

impl Phase {
    pub fn new(id: PhaseId, min_seconds: u8, max_seconds: u8, pass_seconds: u8) -> Self {
        assert!(min_seconds > 0);
        assert!(max_seconds >= min_seconds);
        assert!(pass_seconds > 0);

        Self {
            id,
            min_seconds,
            max_seconds,
            pass_seconds,
            yellow_seconds: 3,
            red_seconds: 1,
        }
    }
}

/*
 * A phase set is defined as any unique combination of
 * vehicle or pedestrian phases that occur at the same time.
 *
 * Two phases are said to be simultaneous phases when they operate together for the
 * same period of time. When one of the phases turns green, so does the other phase, and
 * when one of the phases turns yellow, so does the other phase.
 * Two phases are said to be concurrent phases when they operate together for some
 * period of time, but not necessarily the same period of time
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhaseSetId(u8);

#[derive(Clone)]
pub struct PhaseSet {
    id: PhaseSetId,
    phases: Vec<PhaseId>,
}

impl PhaseSet {
    pub fn new(id: u8, first: PhaseId, second: PhaseId) -> Self {
        Self {
            id: PhaseSetId(id),
            phases: vec![first, second],
        }
    }
}

/*
 * u8 bits represents phases 1-8
 * each bit indicates whether recall is active
 * for that phase (bit position)
 */
struct TimingChart {
    /*
     * Vehicle recall is a demand for the green indication that does not require a vehicle to
     * be present while pedestrian recall is a demand for the walk indication that does not
     * require pushing of the pedestrian button. One or more phases can be placed on recall
     * through programming of the signal controller. When recall has been activated for a
     * phase, a call will be placed on the phase only when the phase does not currently have
     * the green
     */

    /*
     * Minimum recall places a call on a phase and then keeps the phase green for at least
     * as long as the initial (or minimum) setting programmed in the signal controller. The
     * phase's green interval may be longer than the initial if actual vehicles continue to
     * activate the vehicle detector and extend the green, but it will never be less than the
     * initial setting.
     */
    min_recall: u8,
    // Remaining recall notes are preserved for later implementation.

    /*
     * Maximum recall places a call on a phase and then keeps the phase green for a time
     * equal to the maximum setting programmed in the signal controller. Unless there is no
     * other demand at the intersection, the green interval for the phase on maximum recall
     * will exactly equal the maximum setting, it will not be any less or any greater. If there
     * is no other conflicting demand at the intersection once the maximum interval has fully
     * timed, the green time will continue to be displayed to the recall phases until such
     * demand occurs. In this case, the green time experienced by the recall phases will be
     * longer than their maximum green setting. The recall phases are said to rest in green
     * during the period when the actual green time exceeds the maximum green time
     */
    // max_recall: u8,

    /*
     * Pedestrian recalls, Activating ped recall for a phase
     * places a continuous call for pedestrian service on that phase. At the appropriate time
     * in the phase rotation, the pedestrian call will be serviced through display of the walk
     * and pedestrian clearance intervals.
     */
    // ped_recall: u8,

    /*
     * Soft recall is almost the same as minimum recall, with one important difference: soft
     * recall will discontinue its call for the green if there are conflicting calls at the
     * intersection generated by actual vehicle demand. The end result is that phases set to
     * soft recall can be skipped if there is no actual demand for the recall phase. With
     * minimum recall, the recall phases are never skipped, they are always serviced even if
     * there is no actual demand
     */
    // soft_recall: u8,
    // ... more?
}

impl TimingChart {
    fn new() -> Self {
        Self { min_recall: 0 }
    }
}

#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interval {
    Red,
    Green,
    Yellow,
    RedClearance,
}

struct RingState {
    phase: PhaseId,
    interval: Interval,
    elapsed_seconds: u32,
    gap_remaining: u32,
    maximum_elapsed: Option<u32>,
    ready: bool,
}

impl RingState {
    fn new(phase: PhaseId) -> Self {
        Self {
            phase,
            interval: Interval::Red,
            elapsed_seconds: 0,
            gap_remaining: 0,
            maximum_elapsed: None,
            ready: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntersectionId(usize);

#[wasm_bindgen]
pub struct Controller {
    id: IntersectionId,
    phases: Vec<Phase>,
    // A traffic signal cycle is one complete set of signal phase sets.
    // also can represent, phase rotation (phasing sequence): order of phase sets on conflicting calls
    rotation: Vec<PhaseSet>,
    timing_chart: TimingChart,

    // external monitor checks controller for bad output (conflicts)
    // if detected, an external monitor can put the whole system into flash
    // making all phases flash a yellow light. (conflict flash)
    //
    // can also be set manually onsite (manual flash)
    //
    // can be programmed in controller to flash during low-volume periods
    // (program flash)
    // flash: bool,
    // A flash plan needs per-approach indications; all-yellow is not a universal conflict response.

    // start up time in seconds to display red light on all phases
    startup_seconds: u8,
    startup_remaining: u32,
    // manual control of the phase set timing by input
    // manual: bool,
    rings: [RingState; 2],
    current_set: usize,
    /*
     * For a phase to be serviced it must first receive a call for service. A vehicle call is a
     * demand for the green interval of a vehicle phase while a pedestrian call is a demand
     * for the walk interval of a pedestrian phase
     */
    pending_calls: u8,
    elapsed_seconds: u32,
}

impl Controller {
    pub fn new(id: usize, phases: Vec<Phase>, rotation: Vec<PhaseSet>) -> Self {
        assert_eq!(phases.len(), 8, "Configure phases 1 through 8");

        for number in 1..=8 {
            let count = phases.iter().filter(|phase| phase.id.0 == number).count();
            assert_eq!(count, 1, "Each phase must be configured exactly once");
        }

        let pairs = [[1, 5], [2, 6], [3, 7], [4, 8]];
        assert_eq!(rotation.len(), pairs.len());

        for (set, pair) in rotation.iter().zip(pairs) {
            assert_eq!(set.phases.len(), 2);
            assert_eq!(set.phases[0].0, pair[0]);
            assert_eq!(set.phases[1].0, pair[1]);
        }

        let rings = [
            RingState::new(rotation[0].phases[0]),
            RingState::new(rotation[0].phases[1]),
        ];
        let startup_seconds = 5;

        Self {
            id: IntersectionId(id),
            phases,
            rotation,
            timing_chart: TimingChart::new(),
            startup_seconds,
            startup_remaining: u32::from(startup_seconds),
            rings,
            current_set: 0,
            pending_calls: 0,
            elapsed_seconds: 0,
        }
    }

    fn phase(&self, id: PhaseId) -> &Phase {
        self.phases.iter().find(|phase| phase.id == id).unwrap()
    }

    fn set_mask(&self, index: usize) -> u8 {
        self.rotation[index]
            .phases
            .iter()
            .fold(0, |mask, phase| mask | phase.bit())
    }

    fn demand(&self) -> u8 {
        self.pending_calls | self.timing_chart.min_recall
    }

    fn has_conflicting_call(&self) -> bool {
        self.demand() & !self.set_mask(self.current_set) != 0
    }

    fn start_phase_set(&mut self, index: usize) {
        self.current_set = index;
        self.pending_calls &= !self.set_mask(index);
        let conflicting_call = self.has_conflicting_call();

        for (ring, &phase) in self.rings.iter_mut().zip(&self.rotation[index].phases) {
            *ring = RingState::new(phase);
            ring.interval = Interval::Green;

            if conflicting_call {
                ring.maximum_elapsed = Some(0);
            }
        }
    }

    fn start_interval(&mut self, interval: Interval) {
        for ring in &mut self.rings {
            ring.interval = interval;
            ring.elapsed_seconds = 0;
        }
    }

    fn next_phase_set(&self) -> usize {
        for offset in 1..=self.rotation.len() {
            let index = (self.current_set + offset) % self.rotation.len();
            if self.demand() & self.set_mask(index) != 0 {
                return index;
            }
        }

        self.current_set
    }

    fn step_green(&mut self) {
        let conflicting_call = self.has_conflicting_call();

        for index in 0..self.rings.len() {
            let phase = self.phase(self.rings[index].phase).clone();
            let ring = &mut self.rings[index];

            ring.gap_remaining = ring.gap_remaining.saturating_sub(1);

            if !conflicting_call || ring.ready {
                continue;
            }

            let maximum = ring.maximum_elapsed.get_or_insert(0);
            *maximum += 1;

            if ring.elapsed_seconds >= u32::from(phase.min_seconds) {
                ring.ready = ring.gap_remaining == 0 || *maximum >= u32::from(phase.max_seconds);
            }
        }

        if self.rings.iter().all(|ring| ring.ready) {
            self.start_interval(Interval::Yellow);
        }
    }

    fn clearance_seconds(&self, interval: Interval) -> u32 {
        self.rings
            .iter()
            .map(|ring| {
                let phase = self.phase(ring.phase);
                match interval {
                    Interval::Yellow => u32::from(phase.yellow_seconds),
                    _ => u32::from(phase.red_seconds),
                }
            })
            .max()
            .unwrap()
    }
}

#[wasm_bindgen]
impl Controller {
    pub fn demo() -> Self {
        let phases = (1..=8)
            .map(|number| {
                let id = PhaseId::new(number).unwrap();

                if number % 2 == 0 {
                    Phase::new(id, 8, 25, 3)
                } else {
                    Phase::new(id, 5, 15, 3)
                }
            })
            .collect();

        let rotation = vec![
            PhaseSet::new(1, PhaseId(1), PhaseId(5)),
            PhaseSet::new(2, PhaseId(2), PhaseId(6)),
            PhaseSet::new(3, PhaseId(3), PhaseId(7)),
            PhaseSet::new(4, PhaseId(4), PhaseId(8)),
        ];

        let mut controller = Controller::new(0, phases, rotation);
        controller.timing_chart.min_recall = u8::MAX;
        controller
    }

    // One call advances one simulated second.
    pub fn step(&mut self) {
        self.elapsed_seconds += 1;

        if self.startup_remaining > 0 {
            self.startup_remaining -= 1;
            if self.startup_remaining == 0 {
                self.start_phase_set(0);
            }
            return;
        }

        for ring in &mut self.rings {
            ring.elapsed_seconds += 1;
        }

        match self.rings[0].interval {
            Interval::Green => self.step_green(),
            Interval::Yellow => {
                if self.rings[0].elapsed_seconds >= self.clearance_seconds(Interval::Yellow) {
                    self.start_interval(Interval::RedClearance);
                }
            }
            Interval::RedClearance => {
                if self.rings[0].elapsed_seconds >= self.clearance_seconds(Interval::RedClearance) {
                    self.start_phase_set(self.next_phase_set());
                }
            }
            Interval::Red => {}
        }
    }

    // A red/yellow call latches. A green hit restarts passage time.
    pub fn call(&mut self, number: u8) -> bool {
        let Some(id) = PhaseId::new(number) else {
            return false;
        };
        let passage = u32::from(self.phase(id).pass_seconds);

        for ring in &mut self.rings {
            if ring.phase == id && ring.interval == Interval::Green {
                if !ring.ready {
                    ring.gap_remaining = passage;
                }
                return true;
            }
        }

        self.pending_calls |= id.bit();

        if self.startup_remaining == 0 && self.rings[0].interval == Interval::Green {
            for ring in &mut self.rings {
                ring.maximum_elapsed.get_or_insert(0);
            }
        }

        true
    }

    pub fn set_min_recall(&mut self, mask: u8) {
        self.timing_chart.min_recall = mask;

        if self.startup_remaining == 0 && self.rings[0].interval == Interval::Green {
            let conflicting_call = self.has_conflicting_call();
            for ring in &mut self.rings {
                if conflicting_call {
                    ring.maximum_elapsed.get_or_insert(0);
                } else {
                    ring.maximum_elapsed = None;
                    ring.ready = false;
                }
            }
        }
    }

    pub fn signal(&self, number: u8) -> Interval {
        for ring in &self.rings {
            if ring.phase.0 == number {
                return ring.interval;
            }
        }

        Interval::Red
    }

    pub fn pending_calls(&self) -> u8 {
        self.pending_calls
    }

    pub fn time(&self) -> u32 {
        self.elapsed_seconds
    }

    pub fn configure_phase(&mut self, number: u8, min: u8, max: u8, pass: u8, yellow: u8, red: u8) {
        let Some(id) = PhaseId::new(number) else {
            return;
        };

        if let Some(phase) = self.phases.iter_mut().find(|phase| phase.id == id) {
            phase.min_seconds = min;
            phase.max_seconds = max;
            phase.pass_seconds = pass;
            phase.yellow_seconds = yellow;
            phase.red_seconds = red;
        }
    }

    pub fn reset(&mut self) {
        let mut controller = Controller::new(self.id.0, self.phases.clone(), self.rotation.clone());
        controller.timing_chart.min_recall = self.timing_chart.min_recall;
        *self = controller;
    }

    pub fn render(&self) -> String {
        self.to_string()
    }

    pub fn render_timing(&self) -> String {
        let mut output = String::from("VEHICLE TIMING / SECONDS\nPHASE   ");
        for number in 1..=8 {
            output.push_str(&format!("{number:>3} "));
        }
        output.push('\n');
        let rows = ["MIN GRN", "MAX GRN", "PASS", "YELLOW", "RED CLR"];

        for (row, label) in rows.iter().enumerate() {
            output.push_str(&format!("{label:<8}"));
            for number in 1..=8 {
                let phase = self.phase(PhaseId(number));
                let values = [
                    phase.min_seconds,
                    phase.max_seconds,
                    phase.pass_seconds,
                    phase.yellow_seconds,
                    phase.red_seconds,
                ];
                output.push_str(&format!("{:>3} ", values[row]));
            }
            output.push('\n');
        }

        output.push_str(&format!("STARTUP {}s\n", self.startup_seconds));
        output
    }
}

impl fmt::Display for Controller {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "CONTROLLER {}  TIME {}s",
            self.id.0, self.elapsed_seconds
        )?;
        if self.startup_remaining > 0 {
            writeln!(f, "STARTUP ALL RED  {}s", self.startup_remaining)?;
        } else {
            writeln!(f, "PHASE SET {}", self.rotation[self.current_set].id.0)?;
        }

        write!(f, "PHASE ")?;
        for number in 1..=8 {
            write!(f, "{number:>4}")?;
        }

        write!(f, "\nLAMP  ")?;
        for number in 1..=8 {
            let lamp = match self.signal(number) {
                Interval::Green => 'G',
                Interval::Yellow => 'Y',
                _ => 'R',
            };
            write!(f, "{lamp:>4}")?;
        }

        write!(f, "\nCALL  ")?;
        for number in 1..=8 {
            let call = if self.pending_calls & PhaseId(number).bit() != 0 {
                'X'
            } else {
                '.'
            };
            write!(f, "{call:>4}")?;
        }

        write!(f, "\nRECALL")?;
        for number in 1..=8 {
            let recall = if self.timing_chart.min_recall & PhaseId(number).bit() != 0 {
                'X'
            } else {
                '.'
            };
            write!(f, "{recall:>4}")?;
        }

        writeln!(f)?;
        writeln!(
            f,
            "\n{:>4} {:>6}  {:<14} {:>7}  {:>3}  {:>3}  {:>3}  READY",
            "RING", "PHASE", "INTERVAL", "ELAPSED", "MIN", "GAP", "MAX",
        )?;
        for (index, ring) in self.rings.iter().enumerate() {
            let phase = self.phase(ring.phase);
            let interval = match ring.interval {
                Interval::Red => "Red",
                Interval::Green => "Green",
                Interval::Yellow => "Yellow",
                Interval::RedClearance => "RedClearance",
            };
            let green = ring.interval == Interval::Green;
            let min = if green {
                u32::from(phase.min_seconds)
                    .saturating_sub(ring.elapsed_seconds)
                    .to_string()
            } else {
                String::from("-")
            };
            let gap = if green {
                ring.gap_remaining.to_string()
            } else {
                String::from("-")
            };
            let max = if green {
                match ring.maximum_elapsed {
                    Some(elapsed) => u32::from(phase.max_seconds)
                        .saturating_sub(elapsed)
                        .to_string(),
                    None => String::from("-"),
                }
            } else {
                String::from("-")
            };
            writeln!(
                f,
                "{:>4} {:>6}  {:<14} {:>7}  {:>3}  {:>3}  {:>3}  {}",
                index + 1,
                ring.phase.0,
                interval,
                ring.elapsed_seconds,
                min,
                gap,
                max,
                if ring.ready { "YES" } else { "NO" },
            )?;
        }

        Ok(())
    }
}
