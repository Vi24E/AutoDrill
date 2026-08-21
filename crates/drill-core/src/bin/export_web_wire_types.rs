use std::path::PathBuf;

use drill_core::{
    AnswerInputInterface, AnswerNode, GenerateWorksheetRequest, ProblemSetIdentity, WorksheetWire,
};
use ts_rs::{Config, TS};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: export_web_wire_types <output-directory>")?;
    std::fs::create_dir_all(&output)?;

    let config = Config::new().with_out_dir(&output);

    // A small set of boundary roots is enough: export_all recursively emits
    // every canonical Rust dependency used by these wire payloads.
    WorksheetWire::export_all(&config)?;
    ProblemSetIdentity::export_all(&config)?;
    AnswerNode::export_all(&config)?;
    AnswerInputInterface::export_all(&config)?;
    GenerateWorksheetRequest::export_all(&config)?;
    drill_core::GradeResultWire::export_all(&config)?;
    Ok(())
}
