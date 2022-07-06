//! build customized error model by giving a name
//! 

use super::simulator::*;
use serde::{Serialize};
use super::types::*;
use super::util_macros::*;
use super::error_model::*;
use super::clap::{ArgEnum, PossibleValue};
use super::code_builder::*;
use std::sync::{Arc};

/// commonly used error models
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum, Serialize, Debug)]
pub enum ErrorModelBuilder {
    /// add data qubit errors and measurement errors individually
    Phenomenological,
    /// tailored surface code with Bell state initialization (logical |+> state) to fix 3/4 of all stabilizers
    TailoredScBellInitPhenomenological,
    TailoredScBellInitCircuit,
    /// arXiv:2104.09539v1 Sec.IV.A
    GenericBiasedWithBiasedCX,
    /// arXiv:2104.09539v1 Sec.IV.A
    GenericBiasedWithStandardCX,
    /// 100% erasure errors only on the data qubits before the gates happen and on the ancilla qubits before the measurement
    ErasureOnlyPhenomenological,
    /// errors happen at 4 stages in each measurement round (although removed errors happening at initialization and measurement stage, measurement errors can still occur when curtain error applies on the ancilla after the last gate)
    OnlyGateErrorCircuitLevel,
}

impl ErrorModelBuilder {
    pub fn possible_values<'a>() -> impl Iterator<Item = PossibleValue<'a>> {
        Self::value_variants().iter().filter_map(ArgEnum::to_possible_value)
    }

    /// apply error model
    pub fn apply(&self, simulator: &mut Simulator, error_model: &mut ErrorModel, error_model_configuration: &serde_json::Value, p: f64, bias_eta: f64, pe: f64) {
        // commonly used biased qubit error node
        let px = p / (1. + bias_eta) / 2.;
        let py = px;
        let pz = p - 2. * px;
        let mut biased_node = ErrorModelNode::new();
        biased_node.pauli_error_rates.error_rate_X = px;
        biased_node.pauli_error_rates.error_rate_Y = py;
        biased_node.pauli_error_rates.error_rate_Z = pz;
        biased_node.erasure_error_rate = pe;
        let biased_node = Arc::new(biased_node);
        // commonly used pure measurement error node
        let mut pm = p;
        if let Some(value) = error_model_configuration.get("measurement_error_rate") {
            pm = value.as_f64().expect("measurement_error_rate must be `f64`");
        }
        let mut pure_measurement_node = ErrorModelNode::new();
        pure_measurement_node.pauli_error_rates.error_rate_Y = pm;  // Y error will cause pure measurement error for StabX (X basis), StabZ (Z basis), StabY (X basis)
        let pure_measurement_node = Arc::new(pure_measurement_node);
        // commonly used noiseless error node
        let noiseless_node = Arc::new(ErrorModelNode::new());
        // error model builder
        match self {
            ErrorModelBuilder::Phenomenological => {
                let simulator = &*simulator;  // force simulator to be immutable, to avoid unexpected changes
                assert!(px + py + pz <= 1. && px >= 0. && py >= 0. && pz >= 0.);
                assert!(pe == 0.);  // phenomenological error model doesn't support erasure errors
                if simulator.measurement_cycles == 1 {
                    eprintln!("[warning] setting error rates of unknown code, no perfect measurement protection is enabled");
                }
                simulator_iter_real!(simulator, position, node, {
                    error_model.set_node(position, Some(noiseless_node.clone()));  // clear existing noise model
                    if position.t < simulator.height - simulator.measurement_cycles {  // no error at the final perfect measurement round
                        if position.t % simulator.measurement_cycles == 0 && node.qubit_type == QubitType::Data {
                            error_model.set_node(position, Some(biased_node.clone()));
                        }
                        if (position.t + 1) % simulator.measurement_cycles == 0 && node.qubit_type != QubitType::Data {  // measurement error must happen before measurement round
                            error_model.set_node(position, Some(pure_measurement_node.clone()));
                        }
                    }
                });
            },
            ErrorModelBuilder::TailoredScBellInitPhenomenological => {
                let (noisy_measurements, dp, dn) = match simulator.code_type {
                    CodeType::RotatedTailoredCode{ noisy_measurements, dp, dn } => { (noisy_measurements, dp, dn) }
                    _ => unimplemented!("tailored surface code with Bell state initialization is only implemented for open-boundary rotated tailored surface code")
                };
                assert!(noisy_measurements > 0, "to simulate bell initialization, noisy measurement must be set +1 (e.g. set noisy measurement 1 is equivalent to 0 noisy measurements)");
                assert!(simulator.measurement_cycles > 1);
                // change all stabilizers at the first round as virtual
                simulator_iter_mut!(simulator, position, node, t => simulator.measurement_cycles, {
                    if node.qubit_type != QubitType::Data {
                        assert!(node.gate_type.is_measurement());
                        assert!(node.gate_type.is_single_qubit_gate());
                        // since no peer, just set myself as virtual is ok
                        node.is_virtual = true;
                        error_model.set_node(position, Some(noiseless_node.clone()));  // clear existing noise model
                    }
                });
                let simulator = &*simulator;  // force simulator to be immutable, to avoid unexpected changes
                assert!(px + py + pz <= 1. && px >= 0. && py >= 0. && pz >= 0.);
                assert!(pe == 0.);  // phenomenological error model doesn't support erasure errors
                if simulator.measurement_cycles == 1 {
                    eprintln!("[warning] setting error rates of unknown code, no perfect measurement protection is enabled");
                }
                // create an error model that is always 50% change of measurement error
                let mut messed_measurement_node = ErrorModelNode::new();
                messed_measurement_node.pauli_error_rates.error_rate_Y = 0.5;  // Y error will cause pure measurement error for StabX (X basis), StabZ (Z basis), StabY (X basis)
                let messed_measurement_node = Arc::new(messed_measurement_node);
                simulator_iter_real!(simulator, position, node, {
                    error_model.set_node(position, Some(noiseless_node.clone()));  // clear existing noise model
                    if position.t == simulator.measurement_cycles - 1 {
                        for i in 0..((dn+1)/2-1) {
                            for j in 0..(dp+1)/2 {
                                // println!("{:?} {:?} {:?}", position.t, 3 + 2*i + 2*j, dn-1 - 2*i + 2*j);
                                error_model.set_node(&pos!(position.t, 3 + 2*i + 2*j, dn-1 - 2*i + 2*j), Some(messed_measurement_node.clone()));
                            }
                        }
                    } else if position.t >= simulator.measurement_cycles {  // no error before the first round
                        if position.t < simulator.height - simulator.measurement_cycles {  // no error at the final perfect measurement round
                            if position.t % simulator.measurement_cycles == 0 && node.qubit_type == QubitType::Data {
                                error_model.set_node(position, Some(biased_node.clone()));
                            }
                            if (position.t + 1) % simulator.measurement_cycles == 0 && node.qubit_type != QubitType::Data {  // measurement error must happen before measurement round
                                error_model.set_node(position, Some(pure_measurement_node.clone()));
                            }
                        }
                    }
                });
            },
            ErrorModelBuilder::GenericBiasedWithBiasedCX | ErrorModelBuilder::GenericBiasedWithStandardCX => {
                // (here) FIRST qubit: anc; SECOND: data, due to circuit design
                let mut initialization_error_rate = p;  // by default initialization error rate is the same as p
                let mut measurement_error_rate = p;
                let mut config_cloned = error_model_configuration.clone();
                let config = config_cloned.as_object_mut().expect("error_model_configuration must be JSON object");
                config.remove("initialization_error_rate").map(|value| initialization_error_rate = value.as_f64().expect("f64"));
                config.remove("measurement_error_rate").map(|value| measurement_error_rate = value.as_f64().expect("f64"));
                if !config.is_empty() { panic!("unknown keys: {:?}", config.keys().collect::<Vec<&String>>()); }
                // normal biased node
                let mut normal_biased_node = ErrorModelNode::new();
                normal_biased_node.pauli_error_rates.error_rate_X = initialization_error_rate / bias_eta;
                normal_biased_node.pauli_error_rates.error_rate_Z = initialization_error_rate;
                normal_biased_node.pauli_error_rates.error_rate_Y = initialization_error_rate / bias_eta;
                let normal_biased_node = Arc::new(normal_biased_node);
                // CZ gate node
                let mut cphase_node = ErrorModelNode::new();
                cphase_node.correlated_pauli_error_rates = Some(CorrelatedPauliErrorRates::default_with_probability(p / bias_eta));
                cphase_node.correlated_pauli_error_rates.as_mut().unwrap().error_rate_ZI = p;
                cphase_node.correlated_pauli_error_rates.as_mut().unwrap().error_rate_IZ = p;
                let cphase_node = Arc::new(cphase_node);
                // CZ gate with measurement error
                let mut cphase_measurement_error_node: ErrorModelNode = (*cphase_node).clone();
                cphase_measurement_error_node.pauli_error_rates.error_rate_X = initialization_error_rate / bias_eta;
                cphase_measurement_error_node.pauli_error_rates.error_rate_Z = initialization_error_rate;
                cphase_measurement_error_node.pauli_error_rates.error_rate_Y = initialization_error_rate / bias_eta;
                let cphase_measurement_error_node = Arc::new(cphase_measurement_error_node);
                // CX gate node
                let mut cx_node = ErrorModelNode::new();
                cx_node.correlated_pauli_error_rates = Some(CorrelatedPauliErrorRates::default_with_probability(p / bias_eta));
                cx_node.correlated_pauli_error_rates.as_mut().unwrap().error_rate_ZI = p;
                match self {
                    ErrorModelBuilder::GenericBiasedWithStandardCX => {
                        cx_node.correlated_pauli_error_rates.as_mut().unwrap().error_rate_IZ = 0.375 * p;
                        cx_node.correlated_pauli_error_rates.as_mut().unwrap().error_rate_ZZ = 0.375 * p;
                        cx_node.correlated_pauli_error_rates.as_mut().unwrap().error_rate_IY = 0.125 * p;
                        cx_node.correlated_pauli_error_rates.as_mut().unwrap().error_rate_ZY = 0.125 * p;
                    },
                    ErrorModelBuilder::GenericBiasedWithBiasedCX => {
                        cx_node.correlated_pauli_error_rates.as_mut().unwrap().error_rate_IZ = 0.5 * p;
                        cx_node.correlated_pauli_error_rates.as_mut().unwrap().error_rate_ZZ = 0.5 * p;
                    },
                    _ => { }
                }
                let cx_node = Arc::new(cx_node);
                // CX gate with measurement error
                let mut cx_measurement_error_node: ErrorModelNode = (*cx_node).clone();
                cx_measurement_error_node.pauli_error_rates.error_rate_X = initialization_error_rate / bias_eta;
                cx_measurement_error_node.pauli_error_rates.error_rate_Z = initialization_error_rate;
                cx_measurement_error_node.pauli_error_rates.error_rate_Y = initialization_error_rate / bias_eta;
                let cx_measurement_error_node = Arc::new(cx_measurement_error_node);
                // iterate over all nodes
                simulator_iter_real!(simulator, position, node, {
                    // first clear error rate
                    error_model.set_node(position, Some(noiseless_node.clone()));
                    if position.t >= simulator.height - simulator.measurement_cycles {  // no error on the top, as a perfect measurement round
                        continue
                    }
                    // do different things for each stage
                    match position.t % simulator.measurement_cycles {
                        1 => {  // initialization
                            error_model.set_node(position, Some(normal_biased_node.clone()));
                        },
                        0 => {  // measurement
                            // do nothing
                        },
                        _ => {
                            let has_measurement_error = position.t % simulator.measurement_cycles == simulator.measurement_cycles - 1 && node.qubit_type != QubitType::Data;
                            match node.gate_type {
                                GateType::CZGate => {
                                    if node.qubit_type != QubitType::Data {  // this is ancilla
                                        // better check whether peer is indeed data qubit, but it's hard here due to Rust's borrow check
                                        error_model.set_node(position, Some(if has_measurement_error { cphase_measurement_error_node.clone() } else { cphase_node.clone() } ));
                                    }
                                },
                                GateType::CXGateControl => {  // this is ancilla in XZZX code, see arXiv:2104.09539v1
                                    error_model.set_node(position, Some(if has_measurement_error { cx_measurement_error_node.clone() } else { cx_node.clone() } ));
                                },
                                _ => { }
                            }
                        },
                    }
                });
            },
            ErrorModelBuilder::TailoredScBellInitCircuit => {
                let (noisy_measurements, dp, _dn) = match simulator.code_type {
                    CodeType::RotatedTailoredCodeBellInit{ noisy_measurements, dp, dn } => { (noisy_measurements, dp, dn) }
                    _ => unimplemented!("tailored surface code with Bell state initialization is only implemented for open-boundary rotated tailored surface code")
                };
                assert!(noisy_measurements > 0, "to simulate bell initialization, noisy measurement must be set +1 (e.g. set noisy measurement 1 is equivalent to 0 noisy measurements)");
                assert!(simulator.measurement_cycles > 1);
                // change all stabilizers at the first round as virtual
                simulator_iter_mut!(simulator, position, node, t => simulator.measurement_cycles, {  ////[Q] why t=>sim.meas_cycles 
                    if node.qubit_type != QubitType::Data {
                        assert!(node.gate_type.is_measurement());
                        assert!(node.gate_type.is_single_qubit_gate());
                        // since no peer, just set myself as virtual is ok
                        node.is_virtual = true;
                        error_model.set_node(position, Some(noiseless_node.clone()));  // clear existing noise model
                    }
                });
                // a bunch of function for determining qubit type during init, copied from code_builder.rs
                let (di, dj) = (dp, dp);
                let is_real = |i: usize, j: usize| -> bool {
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
                };
                // some criteria for bell init 
                let is_bell_init_anc = |i: usize, j: usize| -> bool { 
                    is_real(i, j) 
                    && i - j < dj - 3 
                    && ((i % 4 == 1 && j % 4 == 0) || (i % 4 == 3 && j % 4 == 2))
                };
                let is_bell_init_unfixed = |i: usize, j: usize| -> bool {
                    is_real(i, j)
                    && ((i % 4 == 0 && j % 4 == 3) || (i % 4 == 2 && j % 4 == 1))
                };

                ////Error nodes for XY code
                let initialization_error_rate = p;
                // normal bias nodes
                let mut normal_biased_node = ErrorModelNode::new();
                normal_biased_node.pauli_error_rates.error_rate_X = initialization_error_rate / bias_eta;
                normal_biased_node.pauli_error_rates.error_rate_Z = initialization_error_rate;
                normal_biased_node.pauli_error_rates.error_rate_Y = initialization_error_rate / bias_eta;
                let normal_biased_node = Arc::new(normal_biased_node);

                // normal bias + cx node (for init)
                let mut normal_biased_with_cx_node = (*normal_biased_node).clone();
                normal_biased_with_cx_node.correlated_pauli_error_rates = Some(CorrelatedPauliErrorRates::default_with_probability(p / bias_eta));
                normal_biased_with_cx_node.correlated_pauli_error_rates.as_mut().unwrap().error_rate_ZI = p;
                normal_biased_with_cx_node.correlated_pauli_error_rates.as_mut().unwrap().error_rate_IZ = 0.5 * p;
                normal_biased_with_cx_node.correlated_pauli_error_rates.as_mut().unwrap().error_rate_ZZ = 0.5 * p;
                let normal_biased_with_cx_node = Arc::new(normal_biased_with_cx_node);

                // biased CX gate node; CX & CY have same error model if using bias-preserving gate
                let mut cx_node = ErrorModelNode::new();
                cx_node.correlated_pauli_error_rates = Some(CorrelatedPauliErrorRates::default_with_probability(p / bias_eta));
                cx_node.correlated_pauli_error_rates.as_mut().unwrap().error_rate_ZI = p;
                cx_node.correlated_pauli_error_rates.as_mut().unwrap().error_rate_IZ = 0.5 * p;
                cx_node.correlated_pauli_error_rates.as_mut().unwrap().error_rate_ZZ = 0.5 * p;
                let cx_node = Arc::new(cx_node);

                // reversed CX gate node, for convinience
                let mut rev_cx_node = ErrorModelNode::new();
                rev_cx_node.correlated_pauli_error_rates = Some(CorrelatedPauliErrorRates::default_with_probability(p / bias_eta));
                rev_cx_node.correlated_pauli_error_rates.as_mut().unwrap().error_rate_IZ = p;
                rev_cx_node.correlated_pauli_error_rates.as_mut().unwrap().error_rate_ZI = 0.5 * p;
                rev_cx_node.correlated_pauli_error_rates.as_mut().unwrap().error_rate_ZZ = 0.5 * p;
                let rev_cx_node = Arc::new(rev_cx_node);

                // CX gate with measurement error
                let mut cx_measurement_error_node: ErrorModelNode = (*cx_node).clone();
                cx_measurement_error_node.pauli_error_rates.error_rate_X = initialization_error_rate / bias_eta;
                cx_measurement_error_node.pauli_error_rates.error_rate_Z = initialization_error_rate;
                cx_measurement_error_node.pauli_error_rates.error_rate_Y = initialization_error_rate / bias_eta;
                let cx_measurement_error_node = Arc::new(cx_measurement_error_node);    

                let simulator = &*simulator;  // force simulator to be immutable, to avoid unexpected changes
                assert!(px + py + pz <= 1. && px >= 0. && py >= 0. && pz >= 0.);
                assert!(pe == 0.);  // phenomenological error model doesn't support erasure errors
                if simulator.measurement_cycles == 1 {
                    eprintln!("[warning] setting error rates of unknown code, no perfect measurement protection is enabled");
                }
                // create an error model that is always 50% change of measurement error
                let mut messed_measurement_node = ErrorModelNode::new();
                messed_measurement_node.pauli_error_rates.error_rate_Y = 0.5;  // Y error will cause pure measurement error for StabX (X basis), StabZ (Z basis), StabY (X basis)
                let messed_measurement_node = Arc::new(messed_measurement_node);

                simulator_iter_real!(simulator, position, node, {
                    error_model.set_node(position, Some(noiseless_node.clone()));  // clear existing noise model
                    if position.t >= simulator.measurement_cycles 
                       && position.t < 2 * simulator.measurement_cycles { // first measurement_cycle is empty, used to set a perfect measurement
                        let (i, j) = (position.i, position.j);
                        match position.t {
                            0 => {
                                // if is_bell_init_anc: normal+cx
                                // else: normal
                                if is_bell_init_anc(i, j) {
                                    error_model.set_node(position, Some(normal_biased_with_cx_node.clone()));
                                } else {
                                    error_model.set_node(position, Some(normal_biased_node.clone()));
                                }
                            },
                            1 | 2 | 3 => {
                                // if is_bell_init_anc: cx
                                if is_bell_init_anc(i, j) {
                                    error_model.set_node(position, Some(cx_node.clone()));
                                }
                            },
                            4 => {
                                // if is_bell_init_anc: rev_cx  
                                if is_bell_init_anc(i, j) {
                                    error_model.set_node(position, Some(rev_cx_node.clone()));
                                }
                            },
                            5 => {
                                // if is_bell_init_anc: cx
                                // if is_bell_init_unfixed: y 
                                if is_bell_init_anc(i, j) {
                                    error_model.set_node(position, Some(cx_measurement_error_node.clone()));
                                }
                                if is_bell_init_unfixed(i, j) {
                                    error_model.set_node(position, Some(messed_measurement_node.clone()));
                                }
                            },
                            _ => {
                                //nothing
                            },
                        }
                    } else if  position.t < simulator.height - simulator.measurement_cycles {  // no error before the first round and at final round
                        // do different things for each stage
                        match position.t % simulator.measurement_cycles {
                            1 => {  // pauli error on qubits
                                error_model.set_node(position, Some(normal_biased_node.clone()));
                            },
                            0 => { // measurement
                                // do nothing

                            },
                            _ => { // gate things
                                let has_measurement_error = position.t % simulator.measurement_cycles == simulator.measurement_cycles - 1 && node.qubit_type != QubitType::Data;// && position.t < (noisy_measurements - 2) * simulator.measurement_cycles - 2;
                                // println!("position.t: {:?}; err: {:?}", position.t, has_measurement_error);
                                if (node.gate_type == GateType::CXGateControl || node.gate_type == GateType::CYGateControl) && node.qubit_type != QubitType::Data { //an ancilla
                                    error_model.set_node(position, Some(if has_measurement_error { cx_measurement_error_node.clone() } else { cx_node.clone() } ))
                                }
                            }
                        }
                    }
                });
            },
            ErrorModelBuilder::ErasureOnlyPhenomenological => {
                assert_eq!(p, 0., "pauli error should be 0 in this error model");
                let mut erasure_node = ErrorModelNode::new();
                // erasure node must have some non-zero pauli error rate for the decoder to work properly
                erasure_node.pauli_error_rates.error_rate_X = 1e-300;  // f64::MIN_POSITIVE ~= 2.22e-308
                erasure_node.pauli_error_rates.error_rate_Z = 1e-300;
                erasure_node.pauli_error_rates.error_rate_Y = 1e-300;
                erasure_node.erasure_error_rate = pe;
                let erasure_node = Arc::new(erasure_node);
                // iterate over all nodes
                simulator_iter_real!(simulator, position, node, {
                    // first clear error rate
                    error_model.set_node(position, Some(noiseless_node.clone()));
                    if position.t >= simulator.height - simulator.measurement_cycles {  // no error on the top, as a perfect measurement round
                        continue
                    }
                    if position.t % simulator.measurement_cycles == 0 {  // add data qubit erasure at the beginning
                        if node.qubit_type == QubitType::Data {
                            error_model.set_node(position, Some(erasure_node.clone()));
                        }
                    } else if position.t % simulator.measurement_cycles == simulator.measurement_cycles - 1 {  // the round before measurement, add erasures
                        if node.qubit_type != QubitType::Data {
                            error_model.set_node(position, Some(erasure_node.clone()));
                        }
                    }
                });
            },
            ErrorModelBuilder::OnlyGateErrorCircuitLevel => {
                assert_eq!(bias_eta, 0.5, "bias not supported yet, please use the default value 0.5");
                let mut initialization_error_rate = 0.;
                let mut measurement_error_rate = 0.;
                let mut use_correlated_erasure = false;
                let mut use_correlated_pauli = false;
                let mut before_pauli_bug_fix = false;
                let mut config_cloned = error_model_configuration.clone();
                let config = config_cloned.as_object_mut().expect("error_model_configuration must be JSON object");
                config.remove("initialization_error_rate").map(|value| initialization_error_rate = value.as_f64().expect("f64"));
                config.remove("measurement_error_rate").map(|value| measurement_error_rate = value.as_f64().expect("f64"));
                config.remove("use_correlated_erasure").map(|value| use_correlated_erasure = value.as_bool().expect("bool"));
                config.remove("use_correlated_pauli").map(|value| use_correlated_pauli = value.as_bool().expect("bool"));
                config.remove("before_pauli_bug_fix").map(|value| before_pauli_bug_fix = value.as_bool().expect("bool"));
                if !config.is_empty() { panic!("unknown keys: {:?}", config.keys().collect::<Vec<&String>>()); }
                // initialization node
                let mut initialization_node = ErrorModelNode::new();
                initialization_node.pauli_error_rates.error_rate_X = initialization_error_rate / 3.;
                initialization_node.pauli_error_rates.error_rate_Z = initialization_error_rate / 3.;
                initialization_node.pauli_error_rates.error_rate_Y = initialization_error_rate / 3.;
                let initialization_node = Arc::new(initialization_node);
                // iterate over all nodes
                simulator_iter_real!(simulator, position, node, {
                    // first clear error rate
                    error_model.set_node(position, Some(noiseless_node.clone()));
                    if position.t >= simulator.height - simulator.measurement_cycles {  // no error on the top, as a perfect measurement round
                        continue
                    }
                    // do different things for each stage
                    match position.t % simulator.measurement_cycles {
                        1 => {  // initialization
                            if node.qubit_type != QubitType::Data {
                                error_model.set_node(position, Some(initialization_node.clone()));
                            }
                        },
                        0 => {  // measurement
                            // do nothing
                        },
                        _ => {
                            // errors everywhere
                            let mut this_position_use_correlated_pauli = false;
                            let mut error_node = ErrorModelNode::new();  // it's perfectly fine to instantiate an error node for each node: just memory inefficient at large code distances
                            if use_correlated_erasure {
                                if node.gate_type.is_two_qubit_gate() {
                                    if node.qubit_type != QubitType::Data {  // this is ancilla
                                        // better check whether peer is indeed data qubit, but it's hard here due to Rust's borrow check
                                        let mut correlated_erasure_error_rates = CorrelatedErasureErrorRates::default_with_probability(0.);
                                        correlated_erasure_error_rates.error_rate_EE = pe;
                                        correlated_erasure_error_rates.sanity_check();
                                        error_node.correlated_erasure_error_rates = Some(correlated_erasure_error_rates);
                                        this_position_use_correlated_pauli = use_correlated_pauli;
                                    }
                                }
                            } else {
                                error_node.erasure_error_rate = pe;
                            }
                            // TODO: update the links
                            // this bug is hard to find without visualization tool...
                            // so I develop such a tool at https://qec.wuyue98.cn/ErrorModelViewer2D.html
                            // to compare: (in url, %20 is space, %22 is double quote)
                            //     https://qec.wuyue98.cn/ErrorModelViewer2D.html?p=0.01&pe=0.05&parameters=--use_xzzx_code%20--error_model%20OnlyGateErrorCircuitLevelCorrelatedErasure%20--error_model_configuration%20%22{\%22use_correlated_pauli\%22:true}%22
                            //     https://qec.wuyue98.cn/ErrorModelViewer2D.html?p=0.01&pe=0.05&parameters=--use_xzzx_code%20--error_model%20OnlyGateErrorCircuitLevelCorrelatedErasure%20--error_model_configuration%20%22{\%22use_correlated_pauli\%22:true,\%22before_pauli_bug_fix\%22:true}%22
                            let mut px_py_pz = if before_pauli_bug_fix {
                                if this_position_use_correlated_pauli { (0., 0., 0.) } else { (p/3., p/3., p/3.) }
                            } else {
                                if use_correlated_pauli { (0., 0., 0.) } else { (p/3., p/3., p/3.) }
                            };
                            if position.t % simulator.measurement_cycles == simulator.measurement_cycles - 1 && node.qubit_type != QubitType::Data {
                                // add additional measurement error
                                // whether it's X axis measurement or Z axis measurement, the additional error rate is always `measurement_error_rate`
                                px_py_pz = ErrorType::combine_probability(px_py_pz, (measurement_error_rate / 2., measurement_error_rate / 2., measurement_error_rate / 2.));
                            }
                            let (px, py, pz) = px_py_pz;
                            error_node.pauli_error_rates.error_rate_X = px;
                            error_node.pauli_error_rates.error_rate_Y = py;
                            error_node.pauli_error_rates.error_rate_Z = pz;
                            if pe > 0. {  // need to set minimum pauli error when this is subject to erasure
                                if error_node.pauli_error_rates.error_rate_X == 0. {
                                    error_node.pauli_error_rates.error_rate_X = 1e-300;  // f64::MIN_POSITIVE ~= 2.22e-308
                                }
                                if error_node.pauli_error_rates.error_rate_Y == 0. {
                                    error_node.pauli_error_rates.error_rate_Y = 1e-300;  // f64::MIN_POSITIVE ~= 2.22e-308
                                }
                                if error_node.pauli_error_rates.error_rate_Z == 0. {
                                    error_node.pauli_error_rates.error_rate_Z = 1e-300;  // f64::MIN_POSITIVE ~= 2.22e-308
                                }
                            }
                            if this_position_use_correlated_pauli {
                                let correlated_pauli_error_rates = CorrelatedPauliErrorRates::default_with_probability(p / 15.);  // 15 possible errors equally probable
                                correlated_pauli_error_rates.sanity_check();
                                error_node.correlated_pauli_error_rates = Some(correlated_pauli_error_rates);
                            }
                            error_model.set_node(position, Some(Arc::new(error_node)));
                        },
                    }
                });
            },
        }
    }

    /// check as strictly as possible, given the user specified json error model description
    pub fn apply_error_model_modifier(simulator : &mut Simulator, error_model: &mut ErrorModel, modifier: &serde_json::Value) -> Result<(), String> {
        if modifier.get("code_type").ok_or(format!("missing field: code_type"))? != &json!(simulator.code_type) {
            return Err(format!("mismatch: code_type"))
        }
        if modifier.get("height").ok_or(format!("missing field: height"))? != &json!(simulator.height) {
            return Err(format!("mismatch: height"))
        }
        if modifier.get("vertical").ok_or(format!("missing field: vertical"))? != &json!(simulator.vertical) {
            return Err(format!("mismatch: vertical"))
        }
        if modifier.get("horizontal").ok_or(format!("missing field: horizontal"))? != &json!(simulator.horizontal) {
            return Err(format!("mismatch: horizontal"))
        }
        // iterate nodes
        let nodes = modifier.get("nodes").ok_or(format!("missing field: nodes"))?.as_array().ok_or(format!("format error: nodes"))?;
        if simulator.nodes.len() != nodes.len() {
            return Err(format!("mismatch: nodes.len()"))
        }
        for t in 0..nodes.len() {
            let nodes_row_0 = nodes[t].as_array().ok_or(format!("format error: nodes[{}]", t))?;
            if nodes_row_0.len() != simulator.nodes[t].len() {
                return Err(format!("mismatch: nodes[{}].len()", t))
            }
            for i in 0..nodes_row_0.len() {
                let nodes_row_1 = nodes_row_0[i].as_array().ok_or(format!("format error: nodes[{}][{}]", t, i))?;
                if nodes_row_1.len() != simulator.nodes[t][i].len() {
                    return Err(format!("mismatch: nodes[{}][{}].len()", t, i))
                }
                for j in 0..nodes_row_1.len() {
                    let node = &nodes_row_1[j];
                    if node.is_null() != simulator.nodes[t][i][j].is_none() {
                        return Err(format!("mismatch: nodes[{}][{}][{}].is_none", t, i, j))
                    }
                    if !node.is_null() {
                        let self_node = simulator.nodes[t][i][j].as_mut().unwrap();  // already checked existance
                        if node.get("position").ok_or(format!("missing field: position"))? != &json!(pos!(t, i, j)) {
                            return Err(format!("mismatch position [{}][{}][{}]", t, i, j))
                        }
                        if node.get("qubit_type").ok_or(format!("missing field: qubit_type"))? != &json!(self_node.qubit_type) {
                            return Err(format!("mismatch [{}][{}][{}]: qubit_type", t, i, j))
                        }
                        if node.get("gate_type").ok_or(format!("missing field: gate_type"))? != &json!(self_node.gate_type) {
                            return Err(format!("mismatch [{}][{}][{}]: gate_type", t, i, j))
                        }
                        if node.get("gate_peer").ok_or(format!("missing field: gate_peer"))? != &json!(self_node.gate_peer) {
                            return Err(format!("mismatch [{}][{}][{}]: gate_peer", t, i, j))
                        }
                        // TODO: user can modify the 'is_virtual' attribute to manually discard a measurement event
                        let is_virtual = node.get("is_virtual").ok_or(format!("missing field: is_virtual"))?.as_bool().ok_or(format!("wrong field: is_virtual"))?;
                        let is_peer_virtual = node.get("is_peer_virtual").ok_or(format!("missing field: is_peer_virtual"))?.as_bool().ok_or(format!("wrong field: is_peer_virtual"))?;
                        assert_eq!(is_virtual, self_node.is_virtual, "is_virtual modification not implemented, needs sanity check");
                        assert_eq!(is_peer_virtual, self_node.is_peer_virtual, "is_peer_virtual modification not implemented, needs sanity check");
                        // then copy error rate data
                        let error_model_node = node.get("error_model").ok_or(format!("missing field: error_model"))?.clone();
                        let error_model_node: ErrorModelNode = serde_json::from_value(error_model_node).map_err(|e| format!("{:?}", e))?;
                        error_model.set_node(&pos!(t, i, j), Some(Arc::new(error_model_node)));
                    }
                }
            }
        }
        Ok(())
    }
}

impl std::str::FromStr for ErrorModelBuilder {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for variant in Self::value_variants() {
            if variant.to_possible_value().unwrap().matches(s, false) {
                return Ok(*variant);
            }
        }
        Err(format!("Invalid variant: {}", s))
    }
}
