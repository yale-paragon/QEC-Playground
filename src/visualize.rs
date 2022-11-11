//! Visualizer
//! 
//! This module helps visualize the circuit, error model, model graph, error patterns, corrections, etc.
//! 

use crate::serde_json;
use std::fs::File;
use crate::serde::{Serialize, Deserialize};
use std::io::{Write, Seek, SeekFrom};
use crate::chrono::Local;
use crate::urlencoding;
#[cfg(feature="python_binding")]
use pyo3::prelude::*;
use std::collections::BTreeSet;


pub trait QecpVisualizer {
    fn component_info(&self, abbrev: bool) -> (String, serde_json::Value);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pyclass)]
pub struct VisualizePosition {
    /// vertical axis, -i is up, +i is down (left-up corner is smallest i,j)
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub i: f64,
    /// horizontal axis, -j is left, +j is right (left-up corner is smallest i,j)
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub j: f64,
    /// time axis, top and bottom (orthogonal to the initial view, which looks at -t direction)
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub t: f64,
}

#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pymethods)]
impl VisualizePosition {
    /// create a visualization position
    #[cfg_attr(feature = "python_binding", new)]
    pub fn new(i: f64, j: f64, t: f64) -> Self {
        Self {
            i, j, t
        }
    }
    #[cfg(feature = "python_binding")]
    fn __repr__(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pyclass)]
pub struct Visualizer {
    /// save to file if applicable
    file: Option<File>,
    /// if waiting for the first case
    empty_cases: bool,
    /// component sealed
    component_done: bool,
    /// names of the components
    #[cfg_attr(feature = "python_binding", pyo3(get))]
    pub component_names: BTreeSet<String>,
}

#[cfg_attr(feature = "python_binding", pyfunction)]
pub fn center_positions(mut positions: Vec<VisualizePosition>) -> Vec<VisualizePosition> {
    if !positions.is_empty() {
        let mut max_i = positions[0].i;
        let mut min_i = positions[0].i;
        let mut max_j = positions[0].j;
        let mut min_j = positions[0].j;
        let mut max_t = positions[0].t;
        let mut min_t = positions[0].t;
        for position in positions.iter_mut() {
            if position.i > max_i { max_i = position.i; }
            if position.j > max_j { max_j = position.j; }
            if position.t > max_t { max_t = position.t; }
            if position.i < min_i { min_i = position.i; }
            if position.j < min_j { min_j = position.j; }
            if position.t < min_t { min_t = position.t; }
        }
        let (ci, cj, ct) = ((max_i + min_i) / 2., (max_j + min_j) / 2., (max_t + min_t) / 2.);
        for position in positions.iter_mut() {
            position.i -= ci;
            position.j -= cj;
            position.t -= ct;
        }
    }
    positions
}

#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pymethods)]
impl Visualizer {

    /// create a new visualizer with target filename and node layout
    #[cfg_attr(feature = "python_binding", new)]
    #[cfg_attr(feature = "python_binding", args(positions = "vec![]", center = "true"))]
    pub fn new(mut filepath: Option<String>) -> std::io::Result<Self> {
        if cfg!(feature = "disable_visualizer") {
            filepath = None;  // do not open file
        }
        let mut file = match filepath {
            Some(filepath) => Some(File::create(filepath)?),
            None => None,
        };
        if let Some(file) = file.as_mut() {
            file.set_len(0)?;  // truncate the file
            file.seek(SeekFrom::Start(0))?;  // move the cursor to the front
            file.write_all(format!("{{\"format\":\"qecp\",\"version\":\"{}\"}}", env!("CARGO_PKG_VERSION")).as_bytes())?;
            file.sync_all()?;
        }
        Ok(Self {
            file,
            empty_cases: true,
            component_names: BTreeSet::new(),
            component_done: false,
        })
    }

    /// add component to the visualizer; each component should be independent
    pub fn add_component(&mut self, component: &impl QecpVisualizer) -> std::io::Result<()> {
        assert!(!self.component_done);
        let abbrev = true;
        if let Some(file) = self.file.as_mut() {
            file.seek(SeekFrom::End(-1))?;  // move the cursor before the ending }
            let (name, component_info) = component.component_info(abbrev);
            file.write_all(format!(",\"{}\":", name).as_bytes())?;
            file.write_all(json!(component_info).to_string().as_bytes())?;
            file.write_all(b"}")?;
            file.sync_all()?;
        }
        Ok(())
    }

    pub fn end_component(&mut self) -> std::io::Result<()> {
        assert!(!self.component_done);
        self.component_done = true;
        if let Some(file) = self.file.as_mut() {
            file.seek(SeekFrom::End(-1))?;  // move the cursor before the ending }
            file.write_all(b",\"cases\":[]}")?;
            file.sync_all()?;
        }
        Ok(())
    }

    pub fn add_case(&mut self, case: serde_json::Value) -> std::io::Result<()> {
        if !self.component_done {
            self.end_component()?;
        }
        if let Some(file) = self.file.as_mut() {
            file.seek(SeekFrom::End(-2))?;  // move the cursor before the ending ]}
            if !self.empty_cases {
                file.write_all(b",")?;
            }
            self.empty_cases = false;
            file.write_all(case.to_string().as_bytes())?;
            file.write_all(b"]}")?;
            file.sync_all()?;
        }
        Ok(())
    }

}

const DEFAULT_VISUALIZE_DATA_FOLDER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/visualize/data/");

// only used locally, because this is compile time directory
pub fn visualize_data_folder() -> String {
    DEFAULT_VISUALIZE_DATA_FOLDER.to_string()
}

#[cfg_attr(feature = "python_binding", pyfunction)]
pub fn static_visualize_data_filename() -> String {
    "qecp_vis.json".to_string()
}

#[cfg_attr(feature = "python_binding", pyfunction)]
pub fn auto_visualize_data_filename() -> String {
    format!("{}.json", Local::now().format("%Y%m%d-%H-%M-%S%.3f"))
}

#[cfg_attr(feature = "python_binding", pyfunction)]
pub fn print_visualize_link_with_parameters(filename: String, parameters: Vec<(String, String)>) {
    let default_port = if cfg!(feature = "python_binding") { 51669 } else { 8069 };
    let mut link = format!("http://localhost:{}?filename={}", default_port, filename);
    for (key, value) in parameters.iter() {
        link.push('&');
        link.push_str(&urlencoding::encode(key));
        link.push('=');
        link.push_str(&urlencoding::encode(value));
    }
    if cfg!(feature = "python_binding") {
        println!("opening link {} (use `fusion_blossom.open_visualizer(filename)` to start a server and open it in browser)", link)
    } else {
        println!("opening link {} (start local server by running ./visualize/server.sh) or call `node index.js <link>` to render locally", link)
    }
}

#[cfg_attr(feature = "python_binding", pyfunction)]
pub fn print_visualize_link(filename: String) {
    print_visualize_link_with_parameters(filename, Vec::new())
}

#[cfg(feature="python_binding")]
#[pyfunction]
pub(crate) fn register(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<VisualizePosition>()?;
    m.add_class::<Visualizer>()?;
    m.add_function(wrap_pyfunction!(static_visualize_data_filename, m)?)?;
    m.add_function(wrap_pyfunction!(auto_visualize_data_filename, m)?)?;
    m.add_function(wrap_pyfunction!(print_visualize_link_with_parameters, m)?)?;
    m.add_function(wrap_pyfunction!(print_visualize_link, m)?)?;
    m.add_function(wrap_pyfunction!(center_positions, m)?)?;
    Ok(())
}
