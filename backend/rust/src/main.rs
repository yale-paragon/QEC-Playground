mod util;
mod test;
mod tool;
mod types;
mod qec;
mod web;
mod blossom_v;
mod mwpm_approx;
mod ftqec;
mod offer_decoder;
mod reproducible_rand;
mod offer_mwpm;
mod union_find_decoder;
mod distributed_uf_decoder;
mod fpga_generator;
mod fast_benchmark;

#[macro_use] extern crate clap;
#[macro_use] extern crate serde_json;
extern crate ndarray;
extern crate rand;
#[cfg(not(feature="noserver"))]
extern crate actix_web;
#[cfg(not(feature="noserver"))]
extern crate actix_cors;
extern crate serde;
extern crate pyo3;
extern crate libc;
extern crate num_cpus;
extern crate petgraph;
extern crate pbr;
extern crate rand_core;
#[macro_use] extern crate derivative;
extern crate union_find;
extern crate derive_more;
extern crate lazy_static;
extern crate either;
extern crate rug;
extern crate shlex;

fn create_clap_parser<'a, 'b>(color_setting: clap::AppSettings) -> clap::App<'a, 'b> {
    clap_app!(QECPlayground =>
        (version: "1.1")
        (author: "Yue Wu (yue.wu@yale.edu), Namitha Liyanage (namitha.liyanage@yale.edu)")
        (setting: clap::AppSettings::VersionlessSubcommands)
        (setting: color_setting)
        (about: "Quantum Error Correction Playground")
        (setting: clap::AppSettings::SubcommandRequiredElseHelp)
        (@subcommand test => (about: "testing features")
            (setting: clap::AppSettings::SubcommandRequiredElseHelp)
            (@subcommand save_load => (about: "testing save and load functionality"))
            (@subcommand perfect_measurement => (about: "print a perfect measurement"))
            (@subcommand validate_correction => (about: "validate x and z correction"))
            (@subcommand naive_correction => (about: "a naive error correction algorithm"))
            (@subcommand try_blossom_correction => (about: "try to use blossom library to decoder"))
            (@subcommand maximum_max_weight_matching_correction => (about: "try to use networkx python library to decoder"))
            (@subcommand debug_tests => (about: "test for debug"))
            (@subcommand archived_debug_tests => (about: "archived debug tests"))
            (@subcommand all => (about: "run all tests"))
            (@subcommand offer_decoder_study => (about: "find some error cases in which offer decoder fails but MWPM decoder succeeds")
                (@arg d: +required "code distance")
                (@arg p: +required "error rate")
                (@arg count: -c --count +takes_value "how many cases to find")
                (@arg max_resend: -r --max_resend +takes_value "maximum rounds to resend offer, default to usize::MAX")
                (@arg max_cycles: -m --max_cycles +takes_value "maximum cycles to run, corresponding to clock cycle in real hardware, default to usize::MAX")
                (@arg print_error_pattern_to_find_infinite_loop: --print_error_pattern_to_find_infinite_loop "print all error patterns")
            )
            (@subcommand offer_algorithm_study => (about: "find some error cases in which offer algorithm fails but MWPM algorithm succeeds")
                (@arg d: +required "code distance")
                (@arg p: +required "error rate")
                (@arg count: -c --count +takes_value "how many cases to find")
                (@arg max_resend: -r --max_resend +takes_value "maximum rounds to resend offer, default to usize::MAX")
                (@arg max_cycles: -m --max_cycles +takes_value "maximum cycles to run, corresponding to clock cycle in real hardware, default to usize::MAX")
                (@arg print_error_pattern_to_find_infinite_loop: --print_error_pattern_to_find_infinite_loop "print all error patterns")
            )
            (@subcommand union_find_decoder_study => (about: "find some error cases in which union find decoder fails but MWPM algorithm succeeds")
                (@arg d: +required "code distance")
                (@arg p: +required "error rate")
                (@arg count: -c --count +takes_value "how many cases to find")
                (@arg max_cost: -m --max_cost +takes_value "maximum cost")
            )
            (@subcommand union_find_decoder_xzzx_code_study => (about: "find some error cases in which union find decoder fails but MWPM algorithm succeeds")
                (@arg d: +required "code distance")
                (@arg p: +required "error rate")
                (@arg count: -c --count +takes_value "how many cases to find")
                (@arg max_half_weight: --max_half_weight +takes_value "maximum weight will be 2 * max_half_weight")
                (@arg bias_eta: --bias_eta +takes_value "bias_eta = pz / (px + py) and px = py, px + py + pz = p. default to 1/2, which is px = pz = py")
            )
            (@subcommand distributed_union_find_decoder_study => (about: "find some error cases in which distributed union find decoder fails but MWPM algorithm succeeds")
                (@arg d: +required "code distance")
                (@arg p: +required "error rate")
                (@arg count: -c --count +takes_value "how many cases to find")
            )
        )
        (@subcommand tool => (about: "tools")
            (setting: clap::AppSettings::SubcommandRequiredElseHelp)
            (@subcommand generate_random_errors => (about: "generate random errors")
                (@arg Ls: +required "[L1,L2,L3,...,Ln]")
                (@arg ps: +required "[p1,p2,p3,...,pm]")
                (@arg N: +required "how many valid samples for each (d,p) config")
                (@arg directory: -d --directory +takes_value "directory to output files, default to ./")
            )
            (@subcommand decoder_benchmark => (about: "test decoder")
                (@arg Ls: +required "[L1,L2,L3,...,Ln]")
                (@arg ps: +required "[p1,p2,p3,...,pm]")
                (@arg directory: -d --directory +takes_value "directory to output files, default to ./")
                (@arg qec_decoder: -q --qec_decoder +takes_value "available decoders, e.g. `naive_decoder`")
            )
            (@subcommand automatic_benchmark => (about: "automatically run benchmark with round upper bound, lower bound and minimum error cases")
                (@arg Ls: +required "[L1,L2,L3,...,Ln]")
                (@arg ps: +required "[p1,p2,p3,...,pm]")
                (@arg max_N: -m --max_N +takes_value "maximum total count, default to 100000000")
                (@arg min_error_cases: -e --min_error_cases +takes_value "minimum error cases, default to 1000")
                (@arg qec_decoder: -q --qec_decoder +takes_value "available decoders, e.g. `naive_decoder`")
            )
            (@subcommand error_rate_MWPM_with_weight => (about: "automatic benchmark on MWPM with weights from file")
                (@arg Ls: +required "[L1,L2,L3,...,Ln]")
                (@arg ps: +required "[p1,p2,p3,...,pm]")
                (@arg max_N: -m --max_N +takes_value "maximum total count, default to 100000000")
                (@arg min_error_cases: -e --min_error_cases +takes_value "minimum error cases, default to 1000")
                (@arg weights: -w --weights +takes_value "path to weights file, e.g. `default_weights.txt`")
                (@arg parallel: -p --parallel +takes_value "how many parallel threads to use. 0 will use number of CPUs - 1. WARNING: this doesn't work well! seems like it has global python locks or so. try to parallel using processes instead! DO NOT USE THIS!")
            )
            (@subcommand fault_tolerant_benchmark => (about: "benchmark fault tolerant algorithm")
                (@arg Ls: +required "[L1,L2,L3,...,Ln] will be code distance of i and j dimension if djs is not provided")
                (@arg djs: --djs +takes_value "[dj1,dj2,dj3,...,djn], will be [L1,L2,L3,...,Ln] if not provided")
                (@arg Ts: +required "[T1,T2,T3,...,Tn], must have exactly the same length as `Ls`")
                (@arg ps: +required "[p1,p2,p3,...,pm] p = px + py + pz")
                (@arg pes: --pes +takes_value "[pe1,pe2,pe3,...,pem] erasure error rate, default to 0")
                (@arg max_N: -m --max_N +takes_value "maximum total count, default to 100000000")
                (@arg min_error_cases: -e --min_error_cases +takes_value "minimum error cases, default to 10000")
                (@arg parallel: -p --parallel +takes_value "how many parallel threads to use. 0 will use number of CPUs - 1")
                (@arg validate_layer: -v --validate_layer +takes_value "validate correction on which layer (all/top/bottom/boundary/<layer>), default to `boundary`")
                (@arg mini_sync_time: --mini_sync_time +takes_value "minimum sync time, default to 0.5s")
                (@arg no_autotune: -n --no_autotune "disable autotune, so that all edges are equally weighted")
                (@arg rotated_planar_code: -r --rotated_planar_code "use rotated planar code instead of standard planar code")
                (@arg ignore_6_neighbors: -i --ignore_6_neighbors "ignore 6 neighbors, so that only straight neighbors are kept")
                (@arg extra_measurement_error: -x --extra_measurement_error +takes_value "the pure measurement error would be p*x, default to 1")
                (@arg bypass_correction: --bypass_correction "bypass correction procedure to test is logical error rate calculation behaving good")
                (@arg independent_px_pz: --independent_px_pz "change the error model to (1-px-pz-pxpz)I + px X + pz Z + pxpz Y")
                (@arg only_count_logical_x: --only_count_logical_x "only count X logical errors but not all logical error. Alert: only available when validate_layer != all")
                (@arg only_count_logical_z: --only_count_logical_z "only count Z logical errors but not all logical error. Alert: only available when validate_layer != all")
                (@arg imperfect_initialization: --imperfect_initialization "if imperfect initialization, then there is bottom boundary because errors happen on the bottom")
                (@arg shallow_error_on_bottom: --shallow_error_on_bottom "add error to data qubit at t=6, so that no measurement error happens at bottom layer. this option can be used with T=0 for single perfect measurement round with only data qubit errors")
                (@arg no_y_error: --no_y_error "set probability of y errors to 0")
                (@arg use_xzzx_code: --use_xzzx_code "use XZZX surface code")
                (@arg bias_eta: --bias_eta +takes_value "bias_eta = pz / (px + py) and px = py, px + py + pz = p. default to 1/2, which is px = pz = py")
                (@arg decoder: --decoder +takes_value "supported decoders: MWPM=MinimumWeightPerfectMatching, UF=UnionFind")
                (@arg max_half_weight: --max_half_weight +takes_value "[UnionFind decoder only] maximum weight will be 2 * max_half_weight")
                // 2022.1.25: change default behavior to use combined probability, this will improve threshold and makes more sense
                (@arg disable_combined_probability: --disable_combined_probability "disable combined probability of matching pairs instead of choosing the maximum probability")
                // 2022.1.25: change default behavior to use ln((1-p)/p) instead of -ln(p), this will improve threshold (although very slightly) but makes more sense
                (@arg disable_autotune_minus_no_error: --disable_autotune_minus_no_error "use ln(1/p) as weight instead of the default ln((1-p)/p)")
                (@arg error_model: --error_model +takes_value "possible error models see types::ErrorModel")
                (@arg error_model_configuration: --error_model_configuration +takes_value "a json object describing the error model details")
                (@arg no_stop_if_next_model_is_not_prepared: -s --no_stop_if_next_model_is_not_prepared "in rough experiment (e.g. estimate the threshold by testing multiple (di,dj,T) configurations) you can use this option to avoid wasting CPU time, as it will not stop current experiment if the model of next experiment is not prepared. Note that you should keep #threads + 1 <= #CPU because the additional thread is for computing the next model")
                (@arg log_runtime_statistics: --log_runtime_statistics +takes_value "log the runtime statistical information, given the path of the log file")
                (@arg detailed_runtime_statistics: --detailed_runtime_statistics "log the detailed runtime statistics if available, leading to much larger log file")
                (@arg log_error_pattern_into_statistics_when_has_logical_error: --log_error_pattern_into_statistics_when_has_logical_error "log the error pattern in the log file, which makes the log file much larger")
                (@arg time_budget: --time_budget +takes_value "for each configuration, give a maximum time to run (in second)")
                (@arg use_fast_benchmark: --use_fast_benchmark "use fast benchmark estimation instead of Monte Carlo method")
                (@arg fbench_disable_additional_error: --fbench_disable_additional_error "do not add additional error when running fast benchmark")
                (@arg fbench_use_fake_decoder: --fbench_use_fake_decoder "fake decoder succeed only if mini_sync_time")
                (@arg fbench_use_simple_sum: --fbench_use_simple_sum +takes_value "by default to false")
                (@arg fbench_assignment_sampling_amount: --fbench_assignment_sampling_amount +takes_value "by default to 1")
                (@arg fbench_weighted_path_sampling: --fbench_weighted_path_sampling "used weighted sampling")
                (@arg fbench_weighted_assignment_sampling: --fbench_weighted_assignment_sampling "use weighted sampling in error assignment")
                (@arg fbench_target_dev: --fbench_target_dev +takes_value "if the deviation of fbench logical error rate is smaller than a number (by default 0 which is never achieved) while keeping for at least 100 rounds, it will exit normally")
                (@arg rug_precision: --rug_precision +takes_value "default to 128, the number of bits in a float number used for fast benchmark")
                (@arg disable_optimize_correction_pattern: --disable_optimize_correction_pattern "disable this optimization")
                // debugging print utilities
                (@arg debug_print_only: --debug_print_only "only print requested information without running the benchmark")
                (@arg debug_print_direct_connections: --debug_print_direct_connections "print direct connections, or model graph in our paper https://www.yecl.org/publications/wu2022qec.pdf")
                (@arg debug_print_exhausted_connections: --debug_print_exhausted_connections "print exhausted connections, or complete model graph in our paper https://www.yecl.org/publications/wu2022qec.pdf")
                (@arg debug_print_error_model: --debug_print_error_model "print error model, without building the exhausted graph")
                // adding features from Fowler's paper
                (@arg use_reduced_graph: --use_reduced_graph "remove edge between two vertices if both of them have smaller weight matching to boundary than matching each other")
            )
            (@subcommand decoder_comparison_benchmark => (about: "benchmark fault tolerant algorithm")
                (@arg Ls: +required "[L1,L2,L3,...,Ln]")
                (@arg Ts: +required "[T1,T2,T3,...,Tn], must have exactly the same length as `Ls`")
                (@arg ps: +required "[p1,p2,p3,...,pm]")
                (@arg max_N: -m --max_N +takes_value "maximum total count, default to 100000000")
                (@arg min_error_cases: -e --min_error_cases +takes_value "minimum error cases, default to 10000")
                (@arg parallel: -p --parallel +takes_value "how many parallel threads to use. 0 will use number of CPUs - 1")
                (@arg validate_layer: -v --validate_layer +takes_value "validate correction on which layer (all/top/bottom/boundary/<layer>), default to `boundary`")
                (@arg mini_batch: -b --mini_batch +takes_value "mini batch, default to 1000")
                (@arg autotune: -a --autotune +takes_value "whether enable autotune, default to true")
                (@arg rotated_planar_code: -r --rotated_planar_code +takes_value "whether use rotated planar code, default to false")
                (@arg ignore_6_neighbors: -i --ignore_6_neighbors +takes_value "whether ignore 6 neighbors, so that only straight neighbors are kept, default to false")
                (@arg extra_measurement_error: -x --extra_measurement_error +takes_value "the pure measurement error would be p*x, default to 1")
                (@arg bypass_correction: --bypass_correction "bypass correction procedure to test is logical error rate calculation behaving good")
                (@arg independent_px_pz: --independent_px_pz "change the error model to (1-px-pz-pxpz)I + px X + pz Z + pxpz Y")
                (@arg only_count_logical_x: --only_count_logical_x "only count X logical errors but not all logical error. Alert: only available when validate_layer != all")
                (@arg imperfect_initialization: --imperfect_initialization "if imperfect initialization, then there is bottom boundary because errors happen on the bottom")
                (@arg substreams: -s --substreams +takes_value "Number of substreams for substream comparison algorithm, default to 32")
            )
            (@subcommand offer_decoder_standard_planar_benchmark => (about: "benchmark offer decoder algorithm with standard planar code")
                (@arg Ls: +required "[L1,L2,L3,...,Ln]")
                (@arg ps: +required "[p1,p2,p3,...,pm]")
                (@arg max_N: -m --max_N +takes_value "maximum total count, default to 100000000")
                (@arg min_error_cases: -e --min_error_cases +takes_value "minimum error cases, default to 10000")
                (@arg parallel: -p --parallel +takes_value "how many parallel threads to use. 0 will use number of CPUs - 1")
                (@arg mini_batch: -b --mini_batch +takes_value "mini batch, default to 1000")
                (@arg only_count_logical_x: --only_count_logical_x "only count X logical errors but not all logical error.")
                (@arg max_resend: -r --max_resend +takes_value "maximum rounds to resend offer, default to usize::MAX")
                (@arg max_cycles: -c --max_cycles +takes_value "maximum cycles to run, corresponding to clock cycle in real hardware, default to usize::MAX")
                (@arg disable_probabilistic_accept: --disable_probabilistic_accept "disable probabilistic accept, this will cause dead lock and degrade performance of d>5")
                (@arg repeat_experiment_each_error: --repeat_experiment_each_error +takes_value "repeat experiment for each error pattern, default to 1")
            )
            (@subcommand offer_algorithm_standard_planar_benchmark => (about: "benchmark offer decoder algorithm with standard planar code")
                (@arg Ls: +required "[L1,L2,L3,...,Ln]")
                (@arg ps: +required "[p1,p2,p3,...,pm]")
                (@arg max_N: -m --max_N +takes_value "maximum total count, default to 100000000")
                (@arg min_error_cases: -e --min_error_cases +takes_value "minimum error cases, default to 10000")
                (@arg parallel: -p --parallel +takes_value "how many parallel threads to use. 0 will use number of CPUs - 1")
                (@arg mini_batch: -b --mini_batch +takes_value "mini batch, default to 1000")
                (@arg only_count_logical_x: --only_count_logical_x "only count X logical errors but not all logical error.")
                (@arg max_resend: -r --max_resend +takes_value "maximum rounds to resend offer, default to usize::MAX")
                (@arg max_cycles: -c --max_cycles +takes_value "maximum cycles to run, corresponding to clock cycle in real hardware, default to usize::MAX")
                (@arg disable_probabilistic_accept: --disable_probabilistic_accept "disable probabilistic accept, this will cause dead lock and degrade performance of d>5")
                (@arg repeat_experiment_each_error: --repeat_experiment_each_error +takes_value "repeat experiment for each error pattern, default to 1")
            )
            (@subcommand union_find_decoder_standard_planar_benchmark => (about: "benchmark union find decoder with standard planar code")
                (@arg Ls: +required "[L1,L2,L3,...,Ln]")
                (@arg ps: +required "[p1,p2,p3,...,pm]")
                (@arg max_N: -m --max_N +takes_value "maximum total count, default to 100000000")
                (@arg min_error_cases: -e --min_error_cases +takes_value "minimum error cases, default to 10000")
                (@arg parallel: -p --parallel +takes_value "how many parallel threads to use. 0 will use number of CPUs - 1")
                (@arg mini_batch: -b --mini_batch +takes_value "mini batch, default to 1000")
                (@arg only_count_logical_x: --only_count_logical_x "only count X logical errors but not all logical error.")
                (@arg no_y_error: --no_y_error "set probability of y errors to 0")
                (@arg towards_mwpm: --towards_mwpm "use advanced methods toward MWPM decoder")
                (@arg max_half_weight: --max_half_weight +takes_value "maximum weight will be 2 * max_half_weight")
                (@arg bias_eta: --bias_eta +takes_value "bias_eta = pz / (px + py) and px = py, px + py + pz = p. default to 1/2, which is px = pz = py")
            )
            (@subcommand distributed_union_find_decoder_standard_planar_benchmark => (about: "benchmark distributed union find decoder with standard planar code")
                (@arg Ls: +required "[L1,L2,L3,...,Ln]")
                (@arg ps: +required "[p1,p2,p3,...,pm]")
                (@arg max_N: -m --max_N +takes_value "maximum total count, default to 100000000")
                (@arg min_error_cases: -e --min_error_cases +takes_value "minimum error cases, default to 10000")
                (@arg parallel: -p --parallel +takes_value "how many parallel threads to use. 0 will use number of CPUs - 1")
                (@arg mini_batch: -b --mini_batch +takes_value "mini batch, default to 1000")
                (@arg only_count_logical_x: --only_count_logical_x "only count X logical errors but not all logical error.")
                (@arg output_cycle_distribution: --output_cycle_distribution "output cycle distribution to a json file")
                (@arg fast_channel_interval: --fast_channel_interval +takes_value "add fast channels at distance (fast_channel_interval ^ k), default to 0 (no fast channel)")
                (@arg no_y_error: --no_y_error "set probability of y errors to 0")
            )
        )
        (@subcommand fpga_generator => (about: "fpga_generator")
            (setting: clap::AppSettings::SubcommandRequiredElseHelp)
            (@subcommand perfect_measurement_distributed_union_find => (about: "DUF decoder under perfect measurement condition")
                (@arg d: +required "code distance")
            )
            (@subcommand fault_tolerant_distributed_union_find => (about: "DUF decoder under imperfect measurement condition")
                (@arg d: +required "code distance")
                (@arg measurement_rounds: +required "measurement rounds")
                (@arg p: +takes_value "physical error rate")
                (@arg autotune: -a --autotune "if set, enable topological code autotune structure")
                (@arg fast_channel_interval: -f --fast_channel_interval +takes_value "fast channel interval, default to 1")
            )
        )
        (@subcommand server => (about: "HTTP server for decoding information")
            (@arg port: -p --port +takes_value "listening on <addr>:<port>, default to 8066")
            (@arg addr: -a --addr +takes_value "listening on <addr>:<port>, default to \"127.0.0.1\"")
            (@arg root_url: -r --root_url +takes_value "root url")
        )
    )
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    let matches = create_clap_parser(clap::AppSettings::ColorAuto).get_matches();

    match matches.subcommand() {
        ("test", Some(matches)) => {
            test::run_matched_test(&matches);
        }
        ("tool", Some(matches)) => {
            let output = tool::run_matched_tool(&matches);
            match output {
                Some(to_print) => { print!("{}", to_print); }
                None => { }
            }
        }
        ("server", Some(matches)) => {
            let port = matches.value_of("port").unwrap_or("8066").to_string().parse::<i32>().unwrap();
            let addr = matches.value_of("addr").unwrap_or("127.0.0.1").to_string();
            let root_url = matches.value_of("root_url").unwrap_or("/").to_string();
            println!("QECP server booting...");
            println!("visit http://{}:{}{}<commands>", addr, port, root_url);
            println!("supported commands include `hello`, `naive_decoder`, etc. See `web.rs` for more commands");
            web::run_server(port, addr, root_url).await?;
        }
        ("fpga_generator", Some(matches)) => {
            fpga_generator::run_matched_fpga_generator(&matches);
        }
        _ => unreachable!()
    }

    Ok(())

}
