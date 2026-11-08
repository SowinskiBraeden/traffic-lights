use wasm_bindgen::prelude::*;

/*
 * Descriptive comments/inspuration sourced from here:
 * https://www.cedengineering.com/userfiles/Introduction%20to%20Traffic%20Signal%20Phasing-R1.pdf
 */

enum Bearing {
    North = 0,
    East = 90,
    South = 180,
    West = 270,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApproachId(u8);
struct Approach {
    id: ApproachId,
    bearing: Bearing, // center outward, e.g. north approach has north outbound lanes, and south inbound lanes
    bearing_offset_deg: i8, // e.g. +/- 15 deg from north
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LaneId(u8);
struct Lane {
    id: LaneId,
    approach_id: ApproachId,
    inbound: bool,
    pedestrian: bool,
}

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
    // convert num 1-8 to a bit representation for the timing tables
    fn bit(self) -> u8 {
        1u8 << (self.0 - 1)
    }
}

/*
 * A phase is any traffic signal display with its
 * own set of timings that controls an individual vehicle or pedestrian movement.
 */
struct Phase {
    id: PhaseId,
    incoming_lane_id: LaneId,
    outgoing_lane_id: LaneId,
    min_seconds: u8,
    max_seconds: u8,

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
    serviced: bool,

    /*
     * For a phase to be serviced it must first receive a call for service. A vehicle call is a
     * demand for the green interval of a vehicle phase while a pedestrian call is a demand
     * for the walk interval of a pedestrian phase
     */
    calls: u8,
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
struct PhaseSet {
    id: PhaseSetId,
    phases: Vec<PhaseId>,
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
    max_recall: u8,

    /*
     * Pedestrian recalls, Activating ped recall for a phase
     * places a continuous call for pedestrian service on that phase. At the appropriate time
     * in the phase rotation, the pedestrian call will be serviced through display of the walk
     * and pedestrian clearance intervals.
     */
    ped_recall: u8,

    /*
     * Soft recall is almost the same as minimum recall, with one important difference: soft
     * recall will discontinue its call for the green if there are conflicting calls at the
     * intersection generated by actual vehicle demand. The end result is that phases set to
     * soft recall can be skipped if there is no actual demand for the recall phase. With
     * minimum recall, the recall phases are never skipped, they are always serviced even if
     * there is no actual demand
     */
    soft_recall: u8,
    // ... more?
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntersectionId(usize);
struct Controller {
    id: IntersectionId,

    phases: Vec<Phase>,

    // A traffic signal cycle is one complete set of signal phase sets.
    // also can represent, phase rotation (phasing sequence): order of phase sets on conflicting calls
    rotation: Vec<PhaseSet>,

    timimg_chart: TimingChart,

    // external monitor checks controller for bad output (conflicts)
    // if detected, an external monitor can put the whole system into flash
    // making all phases flash a yellow light. (conflict flash)
    //
    // can also be set manually onsite (manual flash)
    //
    // can be programmed in controller to flash during low-volume periods
    // (program flash)
    flash: bool,

    // start up time in seconds to display red light on all phases
    startup_seconds: u8,

    // manual control of the phase set timing by input
    manual: bool,
}
