//! # Code Builder
//!
//! Given known a `code_type: CodeType` for a simulator, this will build the proper code.
//! It will ignore `CodeType::Customized` and leave it to user
//!
//! TODO: add svg picture to show example of different code types, see <https://docs.rs/embed-doc-image-showcase/latest/embed_doc_image_showcase/>
//! for how to embed picture in cargo doc
//! 

use super::simulator::*;
use serde::{Serialize, Deserialize};
use super::types::*;
use super::util_macros::*;
use super::clap::{PossibleValue};
use ErrorType::*;


/// commonly used code type that has built-in functions to automatically build up the simulator.
/// other type of code type is also feasible, but one needs to implement the generation of code patch.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum CodeType {
    /// noisy measurement rounds (excluding the final perfect measurement cap), vertical code distance, horizontal code distance
    StandardPlanarCode {
        noisy_measurements: usize,
        di: usize,
        dj: usize,
    },
    /// noisy measurement rounds (excluding the final perfect measurement cap), +i+j axis code distance, +i-j axis code distance
    RotatedPlanarCode {
        noisy_measurements: usize,
        dp: usize,  // positive code distance, +i+j axis, same logical operator with `di`
        dn: usize,  // negative code distance, +i-j axis, same logical operator with `dj`
    },
    /// noisy measurement rounds (excluding the final perfect measurement cap), vertical code distance, horizontal code distance
    StandardXZZXCode {
        noisy_measurements: usize,
        di: usize,
        dj: usize,
    },
    /// noisy measurement rounds (excluding the final perfect measurement cap), +i+j axis code distance, +i-j axis code distance
    RotatedXZZXCode {
        noisy_measurements: usize,
        dp: usize,  // positive code distance, +i+j axis, same logical operator with `di`
        dn: usize,  // negative code distance, +i-j axis, same logical operator with `dj`
    },
    /// noisy measurement rounds (excluding the final perfect measurement cap), vertical code distance, horizontal code distance
    StandardTailoredCode {
        noisy_measurements: usize,
        di: usize,
        dj: usize,
    },
    /// noisy measurement rounds (excluding the final perfect measurement cap), +i+j axis code distance, +i-j axis code distance
    RotatedTailoredCode {
        noisy_measurements: usize,
        dp: usize,  // positive code distance, +i+j axis, same logical operator with `di`
        dn: usize,  // negative code distance, +i-j axis, same logical operator with `dj`
    },
    /// unknown code type, user must provide necessary information and build circuit-level implementation
    Customized,
}

/// built-in code types' information
pub struct BuiltinCodeInformation {
    pub noisy_measurements: usize,
    pub di: usize,
    pub dj: usize,
}

impl CodeType {
    pub fn new(code_type: &String, noisy_measurements: usize, di: usize, dj: usize) -> Self {
        match code_type.as_str() {
            "StandardPlanarCode" => Self::StandardPlanarCode{ noisy_measurements, di, dj },
            "RotatedPlanarCode" => Self::RotatedPlanarCode{ noisy_measurements, dp: di, dn: dj },
            _ => unimplemented!()
        }
    }
    pub fn possible_values<'a>() -> impl Iterator<Item = PossibleValue<'a>> {
        static VARIANTS: &'static [&str] = &[
            "StandardPlanarCode", "RotatedPlanarCode"
        ];
        VARIANTS.iter().map(|x| PossibleValue::new(x))
    }
    pub fn builtin_code_information(&self) -> Option<BuiltinCodeInformation> {
        match &self {
            &CodeType::StandardPlanarCode { noisy_measurements, di, dj } | &CodeType::RotatedPlanarCode { noisy_measurements, dp: di, dn: dj } |
            &CodeType::StandardXZZXCode { noisy_measurements, di, dj } | &CodeType::RotatedXZZXCode { noisy_measurements, dp: di, dn: dj } |
            &CodeType::StandardTailoredCode { noisy_measurements, di, dj } | &CodeType::RotatedTailoredCode { noisy_measurements, dp: di, dn: dj } => {
                Some(BuiltinCodeInformation {
                    noisy_measurements: *noisy_measurements,
                    di: *di,
                    dj: *dj,
                })
            },
            _ => None
        }
    }
}

pub fn build_code(simulator: &mut Simulator) {
    let code_type = &simulator.code_type;
    match code_type {
        &CodeType::StandardPlanarCode { noisy_measurements, di, dj } | &CodeType::RotatedPlanarCode { noisy_measurements, dp: di, dn: dj } => {
            simulator.measurement_cycles = 6;
            assert!(di > 0, "code distance must be positive integer");
            assert!(dj > 0, "code distance must be positive integer");
            let is_rotated = matches!(code_type, CodeType::RotatedPlanarCode { .. });
            if is_rotated {
                assert!(di % 2 == 1, "code distance must be odd integer, current: di = {}", di);
                assert!(dj % 2 == 1, "code distance must be odd integer, current: dj = {}", dj);
            }
            // println!("noisy_measurements: {}, di: {}, dj: {}, is_rotated: {}", noisy_measurements, di, dj, is_rotated);
            let (vertical, horizontal) = if is_rotated {
                (di + dj + 1, di + dj + 1)
            } else {
                (2 * di + 1, 2 * dj + 1)
            };
            let height = simulator.measurement_cycles * (noisy_measurements + 1) + 1;
            // each measurement takes 6 time steps
            let mut nodes = Vec::with_capacity(height);
            let is_real = |i: usize, j: usize| -> bool {
                if is_rotated {
                    let is_real_dj = |pi, pj| { pi + pj < dj || (pi + pj == dj && pi % 2 == 0 && pi > 0) };
                    let is_real_di = |pi, pj| { pi + pj < di || (pi + pj == di && pj % 2 == 0 && pj > 0) };
                    if i <= dj && j <= dj {
                        is_real_dj(dj - i, dj - j)
                    } else if i >= di && j >= di {
                        is_real_dj(i - di, j - di)
                    } else if i >= dj && j <= di {
                        is_real_di(i - dj, di - j)
                    } else if i <= di && j >= dj {
                        is_real_di(di - i, j - dj)
                    } else {
                        unreachable!()
                    }
                } else {
                    i > 0 && j > 0 && i < vertical - 1 && j < horizontal - 1
                }
            };
            let is_virtual = |i: usize, j: usize| -> bool {
                if is_rotated {
                    let is_virtual_dj = |pi, pj| { pi + pj == dj && (pi % 2 == 1 || pi == 0) };
                    let is_virtual_di = |pi, pj| { pi + pj == di && (pj % 2 == 1 || pj == 0) };
                    if i <= dj && j <= dj {
                        is_virtual_dj(dj - i, dj - j)
                    } else if i >= di && j >= di {
                        is_virtual_dj(i - di, j - di)
                    } else if i >= dj && j <= di {
                        is_virtual_di(i - dj, di - j)
                    } else if i <= di && j >= dj {
                        is_virtual_di(di - i, j - dj)
                    } else {
                        unreachable!()
                    }
                } else {
                    if i == 0 || i == vertical - 1 {
                        j % 2 == 1
                    } else if j == 0 || j == horizontal - 1 {
                        i % 2 == 1
                    } else {
                        false
                    }
                }
            };
            let is_present = |i: usize, j: usize| -> bool {
                let is_this_real = is_real(i, j);
                let is_this_virtual = is_virtual(i, j);
                assert!(!(is_this_real && is_this_virtual), "a position cannot be both real and virtual");
                is_this_real || is_this_virtual
            };
            for t in 0..height {
                let mut row_i = Vec::with_capacity(vertical);
                for i in 0..vertical {
                    let mut row_j = Vec::with_capacity(horizontal);
                    for j in 0..horizontal {
                        if is_present(i, j) {
                            let qubit_type = if (i + j) % 2 == 0 {
                                assert!(is_real(i, j), "data qubits should not be virtual");
                                QubitType::Data
                            } else { if i % 2 == 1 { QubitType::StabZ } else { QubitType::StabX } };
                            let mut gate_type = GateType::None;
                            let mut gate_peer = None;
                            match t % simulator.measurement_cycles {
                                1 => {  // initialization
                                    match qubit_type {
                                        QubitType::StabZ => { gate_type = GateType::InitializeZ; }
                                        QubitType::StabX => { gate_type = GateType::InitializeX; }
                                        _ => { }
                                    }
                                },
                                2 => {  // gate 1
                                    if qubit_type == QubitType::Data {
                                        if i+1 < vertical && is_present(i+1, j) {
                                            gate_type = if j % 2 == 1 { GateType::CXGateTarget } else { GateType::CXGateControl };
                                            gate_peer = Some(pos!(t, i+1, j));
                                        }
                                    } else {
                                        if i >= 1 && is_present(i-1, j) {
                                            gate_type = if j % 2 == 1 { GateType::CXGateControl } else { GateType::CXGateTarget };
                                            gate_peer = Some(pos!(t, i-1, j));
                                        }
                                    }
                                },
                                3 => {  // gate 2
                                    if j % 2 == 1 {  // operate with right
                                        if is_present(i, j+1) {
                                            gate_type = GateType::CXGateControl;
                                            gate_peer = Some(pos!(t, i, j+1));
                                        }
                                    } else {  // operate with left
                                        if j >= 1 && is_present(i, j-1) {
                                            gate_type = GateType::CXGateTarget;
                                            gate_peer = Some(pos!(t, i, j-1));
                                        }
                                    }
                                },
                                4 => {  // gate 3
                                    if j % 2 == 1 {  // operate with left
                                        if j >= 1 && is_present(i, j-1) {
                                            gate_type = GateType::CXGateControl;
                                            gate_peer = Some(pos!(t, i, j-1));
                                        }
                                    } else {  // operate with right
                                        if is_present(i, j+1) {
                                            gate_type = GateType::CXGateTarget;
                                            gate_peer = Some(pos!(t, i, j+1));
                                        }
                                    }
                                },
                                5 => {  // gate 4
                                    if qubit_type == QubitType::Data {
                                        if i >= 1 && is_present(i-1, j) {
                                            gate_type = if j % 2 == 1 { GateType::CXGateTarget } else { GateType::CXGateControl };
                                            gate_peer = Some(pos!(t, i-1, j));
                                        }
                                    } else {
                                        if i+1 < vertical && is_present(i+1, j) {
                                            gate_type = if j % 2 == 1 { GateType::CXGateControl } else { GateType::CXGateTarget };
                                            gate_peer = Some(pos!(t, i+1, j));
                                        }
                                    }
                                },
                                0 => {  // measurement
                                    match qubit_type {
                                        QubitType::StabZ => { gate_type = GateType::MeasureZ; }
                                        QubitType::StabX => { gate_type = GateType::MeasureX; }
                                        _ => { }
                                    }
                                },
                                _ => unreachable!()
                            }
                            row_j.push(Some(Box::new(SimulatorNode::new(qubit_type, gate_type, gate_peer.clone()).set_virtual(
                                is_virtual(i, j), gate_peer.map_or(false, |peer| is_virtual(peer.i, peer.j))))));
                        } else {
                            row_j.push(None);
                        }
                    }
                    row_i.push(row_j);
                }
                nodes.push(row_i)
            }
            simulator.vertical = vertical;
            simulator.horizontal = horizontal;
            simulator.height = height;
            simulator.nodes = nodes;
        },
        CodeType::Customized => {
            // skip user customized code
        },
        _ => {
            unimplemented!("code type not supported yet");
        },
    }
}

/// detect common bugs of code building, e.g. peer gate invalid type, is_virtual not correct, etc...
pub fn code_builder_sanity_check(simulator: &Simulator) -> Result<(), String> {
    simulator_iter!(simulator, position, node, {
        // println!("{}", node);
        if node.qubit_type == QubitType::Data {
            if node.gate_type.is_initialization() {
                return Err(format!("data qubit at {} cannot be initialized: gate_type = {:?}", position, node.gate_type))
            }
            if node.gate_type.is_measurement() {
                return Err(format!("data qubit at {} cannot be initialized: gate_type = {:?}", position, node.gate_type))
            }
        }
        match node.gate_peer.as_ref() {
            Some(peer_position) => {
                if node.gate_type.is_single_qubit_gate() {
                    return Err(format!("{} has single qubit gate {:?} should not have peer", position, node.gate_type))
                }
                if !simulator.is_node_exist(&peer_position) {
                    return Err(format!("{}'s peer not exist: {}", position, peer_position))
                }
                let peer_node = simulator.get_node_unwrap(&peer_position);
                match &peer_node.gate_peer {
                    Some(peer_peer_position) => {
                        if peer_peer_position.as_ref() != position {
                            return Err(format!("{}, as the peer of {}, doesn't have correct peer but {}", peer_position, position, peer_peer_position))
                        }
                        if peer_node.gate_type.is_single_qubit_gate() {
                            return Err(format!("{}, as the peer of {}, doesn't have two-qubit gate", peer_position, position))
                        }
                        if node.gate_type.peer_gate() != peer_node.gate_type {
                            return Err(format!("{}, as the peer of {}, doesn't have correct peer gate {:?}, the correct one should be {:?}"
                                , peer_position, position, node.gate_type.peer_gate(), peer_node.gate_type))
                        }
                    },
                    None => {
                        return Err(format!("{}, as the peer of {}, doesn't have peer which is invalid", peer_position, position))
                    }
                }
            }, 
            None => {
                if !node.gate_type.is_single_qubit_gate() {
                    return Err(format!("two qubit gate {:?} should have peer", node.gate_type))
                }
            }
        }
    });
    simulator_iter!(simulator, base_position, _base_node, t => 0, {
        // check that initialization and measurement are always in the same basis
        let mut previous_initialization = GateType::None;
        for t in 1..simulator.height {
            let position = &mut base_position.clone();
            position.t = t;
            let node = simulator.get_node_unwrap(position);
            if node.gate_type.is_initialization() {
                previous_initialization = node.gate_type;
            }
            if node.gate_type.is_measurement() {
                if !node.gate_type.is_corresponding_initialization(&previous_initialization) {
                    return Err(format!("measurement and initialization not in the same basis: node {} has gate type {:?} but previous initialization is {:?}"
                        , position, node.gate_type, previous_initialization))
                }
            }
        }
    });
    Ok(())
}

pub fn code_builder_validate_correction(simulator: &mut Simulator, correction: &SparseCorrection) -> Option<(bool, bool)> {
    // apply the correction directly to the top layer
    let top_t = simulator.height - 1;
    for (position, error) in correction.iter() {
        assert_eq!(position.t, top_t, "correction pattern must only be at top layer");
        let node = simulator.get_node_mut_unwrap(position);
        node.propagated = node.propagated.multiply(error);
    }
    // validate the result
    let code_type = &simulator.code_type;
    let result = match code_type {
        &CodeType::StandardPlanarCode { .. } => {
            // check cardinality of top boundary for logical_i
            let mut top_cardinality = 0;
            for j in (1..simulator.horizontal).step_by(2) {
                let node = simulator.get_node_unwrap(&pos!(top_t, 1, j));
                if node.propagated == Z || node.propagated == Y {
                    top_cardinality += 1;
                }
            }
            let logical_i = top_cardinality % 2 != 0;  // odd cardinality means there is a logical Z error
            // check cardinality of left boundary for logical_j
            let mut left_cardinality = 0;
            for i in (1..simulator.vertical).step_by(2) {
                let node = simulator.get_node_unwrap(&pos!(top_t, i, 1));
                if node.propagated == X || node.propagated == Y {
                    left_cardinality += 1;
                }
            }
            let logical_j = left_cardinality % 2 != 0;  // odd cardinality means there is a logical X error
            Some((logical_i, logical_j))
        },
        _ => None
    };
    // recover the errors
    for (position, error) in correction.iter() {
        let node = simulator.get_node_mut_unwrap(position);
        node.propagated = node.propagated.multiply(error);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use ErrorType::*;

    #[macro_export]
    macro_rules! assert_measurement {
        ($simulator:ident, $errors:expr, $expected_measurements:expr) => {
            $simulator.clear_all_errors();
            for (position, error) in $errors.iter() {
                let node = $simulator.get_node_mut_unwrap(position);
                assert_eq!(node.error, ErrorType::I, "do not set the error at a same position twice: {} {}", position, error);
                node.error = *error;
            }
            $simulator.propagate_errors();
            assert_eq!($simulator.generate_sparse_measurement().to_vec(), $expected_measurements);
        };
    }

    #[test]
    fn code_builder_standard_planar_code() {  // cargo test code_builder_standard_planar_code -- --nocapture
        let di = 7;
        let dj = 5;
        let noisy_measurements = 3;
        let mut simulator = Simulator::new(CodeType::StandardPlanarCode { noisy_measurements, di, dj });
        code_builder_sanity_check(&simulator).unwrap();
        {  // count how many nodes
            let mut nodes_count = 0;
            let mut virtual_nodes_count = 0;
            simulator_iter!(simulator, position, node, {
                // println!("{}", node);
                nodes_count += 1;
                if node.is_virtual {
                    virtual_nodes_count += 1;
                }
            });
            let each_layer_real_node_count = (2 * di - 1) * (2 * dj - 1);
            let each_layer_virtual_node_count = 2 * (di + dj);
            let layer_count = 6 * (noisy_measurements + 1) + 1;
            assert_eq!(nodes_count, layer_count * (each_layer_real_node_count + each_layer_virtual_node_count));
            assert_eq!(virtual_nodes_count, layer_count * each_layer_virtual_node_count);
        }
        {  // check individual qubit type
            {
                let node = simulator.get_node_unwrap(&pos!(0, 0, 1));
                assert_eq!(node.qubit_type, QubitType::StabX);
                assert_eq!(node.gate_type, GateType::MeasureX);
                assert_eq!(node.is_virtual, true);
            }
            {
                let node = simulator.get_node_unwrap(&pos!(0, 0, 2 * dj - 1));
                assert_eq!(node.qubit_type, QubitType::StabX);
                assert_eq!(node.gate_type, GateType::MeasureX);
                assert_eq!(node.is_virtual, true);
            }
            {
                let node = simulator.get_node_unwrap(&pos!(0, 1, 0));
                assert_eq!(node.qubit_type, QubitType::StabZ);
                assert_eq!(node.gate_type, GateType::MeasureZ);
                assert_eq!(node.is_virtual, true);
            }
            {
                let node = simulator.get_node_unwrap(&pos!(0, 2 * di - 1, 0));
                assert_eq!(node.qubit_type, QubitType::StabZ);
                assert_eq!(node.gate_type, GateType::MeasureZ);
                assert_eq!(node.is_virtual, true);
            }
            {
                let node = simulator.get_node_unwrap(&pos!(0, 1, 1));
                assert_eq!(node.qubit_type, QubitType::Data);
                assert_eq!(node.gate_type, GateType::None);
                assert_eq!(node.is_virtual, false);
            }
            {
                let node = simulator.get_node_unwrap(&pos!(0, 1, 2));
                assert_eq!(node.qubit_type, QubitType::StabZ);
                assert_eq!(node.gate_type, GateType::MeasureZ);
                assert_eq!(node.is_virtual, false);
            }
            {
                let node = simulator.get_node_unwrap(&pos!(0, 2, 1));
                assert_eq!(node.qubit_type, QubitType::StabX);
                assert_eq!(node.gate_type, GateType::MeasureX);
                assert_eq!(node.is_virtual, false);
            }
        }
        {  // check gate sequence
            {  // data qubit
                let node = simulator.get_node_unwrap(&pos!(1, 1, 1));
                assert_eq!(node.is_peer_virtual, false);
                assert_eq!(node.gate_type, GateType::None);
                let node = simulator.get_node_unwrap(&pos!(2, 1, 1));
                assert_eq!(node.is_peer_virtual, false);
                assert_eq!(node.gate_type, GateType::CXGateTarget);
                assert_eq!(node.gate_peer.as_ref().map(|x| **x), Some(pos!(2, 2, 1)));
                let node = simulator.get_node_unwrap(&pos!(3, 1, 1));
                assert_eq!(node.is_peer_virtual, false);
                assert_eq!(node.gate_type, GateType::CXGateControl);
                assert_eq!(node.gate_peer.as_ref().map(|x| **x), Some(pos!(3, 1, 2)));
                let node = simulator.get_node_unwrap(&pos!(4, 1, 1));
                assert_eq!(node.is_peer_virtual, true);
                assert_eq!(node.gate_type, GateType::CXGateControl);
                assert_eq!(node.gate_peer.as_ref().map(|x| **x), Some(pos!(4, 1, 0)));
                let node = simulator.get_node_unwrap(&pos!(5, 1, 1));
                assert_eq!(node.is_peer_virtual, true);
                assert_eq!(node.gate_type, GateType::CXGateTarget);
                assert_eq!(node.gate_peer.as_ref().map(|x| **x), Some(pos!(5, 0, 1)));
            }
        }
        {  // check stabilizer measurements
            // data qubit at corner
            assert_measurement!(simulator, [(pos!(0, 1, 1), X)], [pos!(6, 1, 2)]);
            assert_measurement!(simulator, [(pos!(0, 1, 1), Z)], [pos!(6, 2, 1)]);
            assert_measurement!(simulator, [(pos!(0, 1, 1), Y)], [pos!(6, 1, 2), pos!(6, 2, 1)]);
            // data qubit at center
            assert_measurement!(simulator, [(pos!(0, 2, 2), X)], [pos!(6, 1, 2), pos!(6, 3, 2)]);
            assert_measurement!(simulator, [(pos!(0, 2, 2), Z)], [pos!(6, 2, 1), pos!(6, 2, 3)]);
            assert_measurement!(simulator, [(pos!(0, 2, 2), Y)], [pos!(6, 1, 2), pos!(6, 2, 1), pos!(6, 2, 3), pos!(6, 3, 2)]);
            // Z stabilizer measurement error
            assert_measurement!(simulator, [(pos!(5, 1, 2), X)], [pos!(6, 1, 2), pos!(12, 1, 2)]);
            assert_measurement!(simulator, [(pos!(5, 1, 2), Z)], []);  // not sensitive to Z error
            assert_measurement!(simulator, [(pos!(5, 1, 2), Y)], [pos!(6, 1, 2), pos!(12, 1, 2)]);
            // X stabilizer measurement error
            assert_measurement!(simulator, [(pos!(5, 2, 1), X)], []);  // not sensitive to X error
            assert_measurement!(simulator, [(pos!(5, 2, 1), Z)], [pos!(6, 2, 1), pos!(12, 2, 1)]);
            assert_measurement!(simulator, [(pos!(5, 2, 1), Y)], [pos!(6, 2, 1), pos!(12, 2, 1)]);
        }
    }

}
