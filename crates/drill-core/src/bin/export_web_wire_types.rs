use std::path::PathBuf;

use drill_core::{
    AnswerInputInterface, AnswerNode, GenerateWorksheetRequest, ProblemSetIdentity, WorksheetWire,
};
use ts_rs::TS;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: export_web_wire_types <output-directory>")?;
    std::fs::create_dir_all(&output)?;

    // A small set of boundary roots is enough: export_all_to recursively emits
    // every canonical Rust dependency used by these wire payloads.
    WorksheetWire::export_all_to(&output)?;
    ProblemSetIdentity::export_all_to(&output)?;
    AnswerNode::export_all_to(&output)?;
    AnswerInputInterface::export_all_to(&output)?;
    GenerateWorksheetRequest::export_all_to(&output)?;
    drill_core::GradeResultWire::export_all_to(&output)?;
    Ok(())
}
