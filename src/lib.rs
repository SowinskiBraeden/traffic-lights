use std::fmt;
use wasm_bindgen::prelude::*;

/*
 * Descriptive comments/inspuration sourced from here:
 * https://www.cedengineering.com/userfiles/Introduction%20to%20Traffic%20Signal%20Phasing-R1.pdf
 *
 * Dual-ring, single-barrier operation:
 * ring 1: 1 -> 2 | 3 -> 4
 * ring 2: 5 -> 6 | 7 -> 8
 * The vertical line marks the main/side street barrier. Within a barrier group
 * each ring advances on its own (e.g. 1/5 -> 2/5 -> 2/6 is normal), but both
 * rings must park at the barrier before either can cross into the next group.
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
            yellow_seconds: 4,
            red_seconds: 1,
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
    // Also doubles as "parked at the barrier, waiting on the other ring" and
    // the pre-startup state, since both show the same red lamp to a viewer.
    Red,
    Green,
    Yellow,
    RedClearance,
}

// Which side of the barrier a ring is currently serving.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Group {
    A,
    B,
}

impl Group {
    fn flip(self) -> Self {
        match self {
            Group::A => Group::B,
            Group::B => Group::A,
        }
    }
}

// Which of the two phases within the current barrier group a ring is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    First,
    Second,
}

// Ring 1 carries phases 1-4, ring 2 carries phases 5-8; group A is the first
// phase pair in each ring (before the barrier), group B is the second.
fn ring_phase_number(ring_index: usize, group: Group, slot: Slot) -> u8 {
    match (ring_index, group, slot) {
        (0, Group::A, Slot::First) => 1,
        (0, Group::A, Slot::Second) => 2,
        (0, Group::B, Slot::First) => 3,
        (0, Group::B, Slot::Second) => 4,
        (1, Group::A, Slot::First) => 5,
        (1, Group::A, Slot::Second) => 6,
        (1, Group::B, Slot::First) => 7,
        (1, Group::B, Slot::Second) => 8,
        _ => unreachable!("only two rings, two groups and two slots exist"),
    }
}

// Bitmask of the four phases that belong to one barrier group (both rings).
fn group_mask(group: Group) -> u8 {
    let numbers: [u8; 4] = match group {
        Group::A => [1, 2, 5, 6],
        Group::B => [3, 4, 7, 8],
    };
    numbers
        .iter()
        .fold(0, |mask, &number| mask | PhaseId::new(number).unwrap().bit())
}

struct RingState {
    slot: Slot,
    interval: Interval,
    elapsed_seconds: u32,
    gap_remaining: u32,
    maximum_elapsed: Option<u32>,
    ready: bool,
}

impl RingState {
    fn parked() -> Self {
        Self {
            slot: Slot::First,
            interval: Interval::Red,
            elapsed_seconds: 0,
            gap_remaining: 0,
            maximum_elapsed: None,
            ready: false,
        }
    }

    fn green(slot: Slot) -> Self {
        Self {
            slot,
            interval: Interval::Green,
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
    group: Group,
    rings: [RingState; 2],
    /*
     * For a phase to be serviced it must first receive a call for service. A vehicle call is a
     * demand for the green interval of a vehicle phase while a pedestrian call is a demand
     * for the walk interval of a pedestrian phase
     */
    pending_calls: u8,
    elapsed_seconds: u32,
}

impl Controller {
    pub fn new(id: usize, phases: Vec<Phase>) -> Self {
        assert_eq!(phases.len(), 8, "Configure phases 1 through 8");

        for number in 1..=8 {
            let count = phases.iter().filter(|phase| phase.id.0 == number).count();
            assert_eq!(count, 1, "Each phase must be configured exactly once");
        }

        let startup_seconds = 5;

        Self {
            id: IntersectionId(id),
            phases,
            timing_chart: TimingChart::new(),
            startup_seconds,
            startup_remaining: u32::from(startup_seconds),
            group: Group::A,
            rings: [RingState::parked(), RingState::parked()],
            pending_calls: 0,
            elapsed_seconds: 0,
        }
    }

    fn phase(&self, id: PhaseId) -> &Phase {
        self.phases.iter().find(|phase| phase.id == id).unwrap()
    }

    fn demand(&self) -> u8 {
        self.pending_calls | self.timing_chart.min_recall
    }

    // A ring should stop resting and start timing toward ending its current
    // phase only when there's demand it can actually act on: its own next
    // phase within this barrier group, or anything waiting in the other
    // group (since parking to help cross the barrier is the only way to
    // eventually reach it). Demand for the OTHER ring's own succession phase
    // (e.g. phase 2 while this ring sits on phase 5) is not this ring's
    // concern — it can keep resting while the other ring handles it alone.
    fn ring_wants_to_advance(&self, index: usize) -> bool {
        let demand = self.demand();
        let ring = &self.rings[index];

        let own_succession = ring.slot == Slot::First && {
            let second = ring_phase_number(index, self.group, Slot::Second);
            demand & PhaseId::new(second).unwrap().bit() != 0
        };

        own_succession || demand & group_mask(self.group.flip()) != 0
    }

    fn enter_group(&mut self, group: Group) {
        self.group = group;
        self.rings = [RingState::parked(), RingState::parked()];

        for index in 0..2 {
            self.try_resume_ring(index);
        }
    }

    // A parked ring (waiting at the barrier) picks back up as soon as either
    // of its two phases in the current group has demand, skipping straight
    // to the second phase if the first has none (matches real phase-skip
    // behavior rather than always serving the first phase regardless).
    fn try_resume_ring(&mut self, index: usize) {
        if self.rings[index].interval != Interval::Red {
            return;
        }

        let demand = self.demand();
        let first = PhaseId::new(ring_phase_number(index, self.group, Slot::First)).unwrap();
        let second = PhaseId::new(ring_phase_number(index, self.group, Slot::Second)).unwrap();

        if demand & first.bit() != 0 {
            self.rings[index] = RingState::green(Slot::First);
            self.pending_calls &= !first.bit();
        } else if demand & second.bit() != 0 {
            self.rings[index] = RingState::green(Slot::Second);
            self.pending_calls &= !second.bit();
        }
    }

    // Runs the per-ring min/gap/max countdown for every ring still resting
    // in green, independent of whatever the other ring is doing.
    fn update_green_readiness(&mut self) {
        for index in 0..2 {
            if self.rings[index].interval != Interval::Green || self.rings[index].ready {
                continue;
            }

            let wants_to_advance = self.ring_wants_to_advance(index);
            let number = ring_phase_number(index, self.group, self.rings[index].slot);
            let phase = self.phase(PhaseId::new(number).unwrap()).clone();
            let ring = &mut self.rings[index];

            ring.gap_remaining = ring.gap_remaining.saturating_sub(1);

            if !wants_to_advance {
                continue;
            }

            let maximum = ring.maximum_elapsed.get_or_insert(0);
            *maximum += 1;

            if ring.elapsed_seconds >= u32::from(phase.min_seconds) {
                ring.ready = ring.gap_remaining == 0 || *maximum >= u32::from(phase.max_seconds);
            }
        }
    }

    // Advances one ring's own Green -> Yellow -> RedClearance -> (next slot
    // or parked at the barrier) sequence, entirely independent of the other ring.
    fn step_ring(&mut self, index: usize) {
        let number = ring_phase_number(index, self.group, self.rings[index].slot);
        let phase = self.phase(PhaseId::new(number).unwrap()).clone();
        let ring = &mut self.rings[index];

        match ring.interval {
            Interval::Green => {
                if ring.ready {
                    ring.interval = Interval::Yellow;
                    ring.elapsed_seconds = 0;
                }
            }
            Interval::Yellow => {
                if ring.elapsed_seconds >= u32::from(phase.yellow_seconds) {
                    ring.interval = Interval::RedClearance;
                    ring.elapsed_seconds = 0;
                }
            }
            Interval::RedClearance => {
                if ring.elapsed_seconds >= u32::from(phase.red_seconds) {
                    self.advance_ring(index);
                }
            }
            Interval::Red => {}
        }
    }

    // Called once a ring finishes clearing its current phase: either moves on
    // to the group's second phase (if it has demand) or parks at the barrier.
    fn advance_ring(&mut self, index: usize) {
        let slot = self.rings[index].slot;

        if slot == Slot::First {
            let second = ring_phase_number(index, self.group, Slot::Second);
            let id = PhaseId::new(second).unwrap();

            if self.demand() & id.bit() != 0 {
                self.pending_calls &= !id.bit();
                self.rings[index] = RingState::green(Slot::Second);
                return;
            }
        }

        self.rings[index] = RingState::parked();
    }

    // Both rings cross the barrier together only once both have parked, and
    // only if the other side actually has something to serve - otherwise an
    // idle intersection would flip barriers every single tick for no reason.
    fn try_cross_barrier(&mut self) {
        if !self.rings.iter().all(|ring| ring.interval == Interval::Red) {
            return;
        }

        let next = self.group.flip();
        if self.demand() & group_mask(next) != 0 {
            self.enter_group(next);
        }
    }
}

#[wasm_bindgen]
impl Controller {
    pub fn demo() -> Self {
        // Modeled after a Siemens M60 default timing chart: the main street
        // (2/6) carries the most green, the side street (4/8) less, and both
        // left-turn pairs (1/5, 3/7) get short, actuated-only allocations.
        let phases = (1..=8)
            .map(|number| {
                let id = PhaseId::new(number).unwrap();
                let (min, max, pass) = match number {
                    2 | 6 => (15, 50, 4),
                    4 | 8 => (10, 35, 3),
                    1 | 5 => (5, 20, 3),
                    3 | 7 => (5, 15, 3),
                    _ => unreachable!(),
                };

                Phase::new(id, min, max, pass)
            })
            .collect();

        let mut controller = Controller::new(0, phases);
        // Matches the paper's own example: minimum recall on the main-street
        // through phases only, so they rest in green absent conflicting demand.
        // Every other phase is purely actuated and is skipped when uncalled.
        controller.timing_chart.min_recall = PhaseId::new(2).unwrap().bit() | PhaseId::new(6).unwrap().bit();
        controller
    }

    // One call advances one simulated second.
    pub fn step(&mut self) {
        self.elapsed_seconds += 1;

        if self.startup_remaining > 0 {
            self.startup_remaining -= 1;
            if self.startup_remaining == 0 {
                self.enter_group(Group::A);
            }
            return;
        }

        for ring in &mut self.rings {
            ring.elapsed_seconds += 1;
        }

        self.update_green_readiness();

        for index in 0..2 {
            self.step_ring(index);
        }

        self.try_cross_barrier();
    }

    // A red/yellow call latches. A green hit restarts passage time.
    pub fn call(&mut self, number: u8) -> bool {
        let Some(id) = PhaseId::new(number) else {
            return false;
        };

        for index in 0..2 {
            let ring = &self.rings[index];
            if ring.interval != Interval::Green {
                continue;
            }
            if ring_phase_number(index, self.group, ring.slot) != number {
                continue;
            }

            let passage = u32::from(self.phase(id).pass_seconds);
            let ring = &mut self.rings[index];
            if !ring.ready {
                ring.gap_remaining = ring.gap_remaining.max(passage);
            }
            return true;
        }

        self.pending_calls |= id.bit();

        if self.startup_remaining == 0 {
            for index in 0..2 {
                if self.rings[index].interval == Interval::Green && self.ring_wants_to_advance(index) {
                    self.rings[index].maximum_elapsed.get_or_insert(0);
                }
            }
        }

        true
    }

    pub fn set_min_recall(&mut self, mask: u8) {
        self.timing_chart.min_recall = mask;

        if self.startup_remaining > 0 {
            return;
        }

        for index in 0..2 {
            if self.rings[index].interval != Interval::Green {
                continue;
            }
            if self.ring_wants_to_advance(index) {
                self.rings[index].maximum_elapsed.get_or_insert(0);
            } else {
                self.rings[index].maximum_elapsed = None;
                self.rings[index].ready = false;
            }
        }
    }

    pub fn signal(&self, number: u8) -> Interval {
        for index in 0..2 {
            let ring = &self.rings[index];
            if ring.interval == Interval::Red {
                continue;
            }
            if ring_phase_number(index, self.group, ring.slot) == number {
                return ring.interval;
            }
        }

        Interval::Red
    }

    pub fn pending_calls(&self) -> u8 {
        self.pending_calls
    }

    // Seconds the phase has been green, if it is one of the two active rings.
    pub fn ring_elapsed(&self, number: u8) -> Option<u32> {
        for index in 0..2 {
            let ring = &self.rings[index];
            if ring.interval == Interval::Green
                && ring_phase_number(index, self.group, ring.slot) == number
            {
                return Some(ring.elapsed_seconds);
            }
        }

        None
    }

    // Passage time left before this phase's green could gap out, if active.
    pub fn ring_gap(&self, number: u8) -> Option<u32> {
        for index in 0..2 {
            let ring = &self.rings[index];
            if ring.interval == Interval::Green
                && ring_phase_number(index, self.group, ring.slot) == number
            {
                return Some(ring.gap_remaining);
            }
        }

        None
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
        let mut controller = Controller::new(self.id.0, self.phases.clone());
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
            let group = match self.group {
                Group::A => "A",
                Group::B => "B",
            };
            writeln!(f, "BARRIER GROUP {group}")?;
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
            let phase_number = ring_phase_number(index, self.group, ring.slot);
            let phase = self.phase(PhaseId::new(phase_number).unwrap());
            let interval = match ring.interval {
                Interval::Red => "Red",
                Interval::Green => "Green",
                Interval::Yellow => "Yellow",
                Interval::RedClearance => "Clearance",
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
                phase_number,
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
