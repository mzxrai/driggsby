mod demo_text;
mod error_text;
mod format;
mod import_text;
mod intelligence_text;
mod json;
mod mode;
mod schema_text;

use std::io;

use driggsby_client::{ClientError, SuccessEnvelope};

pub use mode::{OutputMode, mode_for_command};

pub fn print_success(success: &SuccessEnvelope, mode: OutputMode) -> io::Result<()> {
    let body = match mode {
        OutputMode::Text => render_text_success(success)?,
        OutputMode::Json => json::render_success_json(success)?,
    };
    println!("{body}");
    Ok(())
}

pub fn print_failure(error: &ClientError, mode: OutputMode) -> io::Result<()> {
    let body = match mode {
        OutputMode::Json => json::render_error_json(error)?,
        OutputMode::Text => error_text::render_error(error),
    };
    println!("{body}");
    Ok(())
}

fn render_text_success(success: &SuccessEnvelope) -> io::Result<String> {
    match success.command.as_str() {
        "schema" => schema_text::render_schema_summary(&success.data),
        "schema.view" => schema_text::render_schema_view(&success.data),
        "import" => import_text::render_import_run(&success.data),
        "import list" => import_text::render_import_list(&success.data),
        "import duplicates" => import_text::render_import_duplicates(&success.data),
        "import keys uniq" => import_text::render_import_keys_uniq(&success.data),
        "import undo" => import_text::render_import_undo(&success.data),
        "demo" | "dash" => demo_text::render_demo_or_dash(&success.command, &success.data),
        "anomalies" => intelligence_text::render_anomalies(&success.data),
        "recurring" => intelligence_text::render_recurring(&success.data),
        _ => Err(io::Error::other(format!(
            "unsupported text output command `{}`",
            success.command
        ))),
    }
}
