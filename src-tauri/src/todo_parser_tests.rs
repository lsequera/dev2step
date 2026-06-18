#[cfg(test)]
mod tests {
    use super::super::todo_parser::*;

    #[test]
    fn test_parsing_and_formatting() {
        let raw = "x (A) 2026-06-17 2026-06-15 Implement CUDA kernel optimization +HPC_Telemetry status:todo due:2026-06-30 id:102";
        let parsed = parse_line(raw, 1).unwrap();
        
        assert_eq!(parsed.id, 102);
        assert_eq!(parsed.priority, Some('A'));
        assert_eq!(parsed.is_completed, true);
        assert_eq!(parsed.completion_date, Some("2026-06-17".to_string()));
        assert_eq!(parsed.creation_date, "2026-06-15".to_string());
        assert_eq!(parsed.description, "Implement CUDA kernel optimization".to_string());
        assert_eq!(parsed.project, Some("HPC_Telemetry".to_string()));
        assert_eq!(parsed.status, TaskStatus::Todo);
        assert_eq!(parsed.due_date, Some("2026-06-30".to_string()));

        let formatted = format_task(&parsed);
        assert!(formatted.contains("x "));
        assert!(formatted.contains("(A)"));
        assert!(formatted.contains("Implement CUDA kernel optimization"));
        assert!(formatted.contains("+HPC_Telemetry"));
        assert!(formatted.contains("status:todo"));
        assert!(formatted.contains("id:102"));
    }

    #[test]
    fn test_parsing_empty_line() {
        assert!(parse_line("", 1).is_err());
        assert!(parse_line("   ", 2).is_err());
    }

    #[test]
    fn test_parsing_no_priority_no_completion() {
        let raw = "2026-06-15 Implement basic features +HPC_Telemetry status:todo id:103";
        let parsed = parse_line(raw, 2).unwrap();
        
        assert_eq!(parsed.id, 103);
        assert_eq!(parsed.priority, None);
        assert_eq!(parsed.is_completed, false);
        assert_eq!(parsed.completion_date, None);
        assert_eq!(parsed.creation_date, "2026-06-15".to_string());
        assert_eq!(parsed.description, "Implement basic features".to_string());
        assert_eq!(parsed.project, Some("HPC_Telemetry".to_string()));
        assert_eq!(parsed.status, TaskStatus::Todo);
        
        let formatted = format_task(&parsed);
        assert_eq!(formatted, "2026-06-15 Implement basic features +HPC_Telemetry status:todo id:103");
    }

    #[test]
    fn test_parsing_defaults_and_status() {
        // No dates, status:icebox (default), no project, no id
        let raw = "Implement basic features parent:103";
        let parsed = parse_line(raw, 3).unwrap();
        
        assert_eq!(parsed.id, 0);
        assert_eq!(parsed.priority, None);
        assert_eq!(parsed.is_completed, false);
        assert_eq!(parsed.completion_date, None);
        assert_eq!(parsed.creation_date, chrono::Local::now().format("%Y-%m-%d").to_string());
        assert_eq!(parsed.description, "Implement basic features".to_string());
        assert_eq!(parsed.project, None);
        assert_eq!(parsed.status, TaskStatus::Icebox);
        assert_eq!(parsed.parent_id, Some(103));
    }
}
