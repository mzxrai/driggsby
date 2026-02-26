use driggsby_client::ClientError;

pub fn render_error(error: &ClientError) -> String {
    let mut lines = vec![
        "Something went wrong, but it's easy to fix.".to_string(),
        String::new(),
        format!("  Error:    {}", error.code),
        format!("  Details:  {}", error.message),
        String::new(),
        "What to do next:".to_string(),
    ];

    if error.recovery_steps.is_empty() {
        lines.push("  1. Retry the command.".to_string());
    } else {
        for (index, step) in error.recovery_steps.iter().enumerate() {
            lines.push(format!("  {}. {step}", index + 1));
        }
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use driggsby_client::ClientError;

    use super::render_error;

    #[test]
    fn renders_standard_error_layout() {
        let error = ClientError::invalid_argument_with_recovery(
            "bad input",
            vec!["run driggsby --help".to_string()],
        );

        let rendered = render_error(&error);
        assert!(rendered.starts_with("Something went wrong, but it's easy to fix."));
        assert!(rendered.contains("  Error:    invalid_argument"));
        assert!(rendered.contains("  Details:  bad input"));
        assert!(rendered.contains("What to do next:"));
        assert!(rendered.contains("  1. run driggsby --help"));
    }
}
