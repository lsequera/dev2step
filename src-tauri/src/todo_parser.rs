use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskStatus {
    Icebox,
    Todo,
    Progress,
    Done,
}

impl TaskStatus {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "todo" => TaskStatus::Todo,
            "progress" => TaskStatus::Progress,
            "done" => TaskStatus::Done,
            _ => TaskStatus::Icebox,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            TaskStatus::Icebox => "icebox",
            TaskStatus::Todo => "todo",
            TaskStatus::Progress => "progress",
            TaskStatus::Done => "done",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DevTask {
    pub id: u32,
    pub priority: Option<char>,
    pub is_completed: bool,
    pub completion_date: Option<String>,
    pub creation_date: String,
    pub description: String,
    pub project: Option<String>,
    pub status: TaskStatus,
    pub due_date: Option<String>,
    pub parent_id: Option<u32>,
    pub line_number: usize,
}

pub fn parse_line(line: &str, line_num: usize) -> Result<DevTask, String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err("Empty line".to_string());
    }

    let mut parts: Vec<&str> = trimmed.split_whitespace().collect();
    let mut is_completed = false;
    if parts[0] == "x" {
        is_completed = true;
        parts.remove(0);
    }

    let mut priority = None;
    if !parts.is_empty() && parts[0].starts_with('(') && parts[0].ends_with(')') && parts[0].len() == 3 {
        let p_char = parts[0].chars().nth(1).unwrap();
        priority = Some(p_char);
        parts.remove(0);
    }

    let mut completion_date = None;
    let date_pattern = |s: &str| s.len() == 10 && s.chars().nth(4) == Some('-') && s.chars().nth(7) == Some('-');
    
    if is_completed && !parts.is_empty() && date_pattern(parts[0]) {
        completion_date = Some(parts[0].to_string());
        parts.remove(0);
    }

    let mut creation_date = chrono::Local::now().format("%Y-%m-%d").to_string();
    if !parts.is_empty() && date_pattern(parts[0]) {
        creation_date = parts[0].to_string();
        parts.remove(0);
    }

    let mut project = None;
    let mut status = TaskStatus::Icebox;
    let mut due_date = None;
    let mut id = None;
    let mut parent_id = None;

    let mut desc_words = Vec::new();

    for word in parts {
        if word.starts_with('+') && word.len() > 1 {
            project = Some(word[1..].to_string());
        } else if word.starts_with("status:") {
            status = TaskStatus::from_str(&word[7..]);
        } else if word.starts_with("due:") {
            due_date = Some(word[4..].to_string());
        } else if word.starts_with("id:") {
            id = word[3..].parse::<u32>().ok();
        } else if word.starts_with("parent:") {
            parent_id = word[7..].parse::<u32>().ok();
        } else {
            desc_words.push(word);
        }
    }

    let description = desc_words.join(" ");
    let final_id = id.unwrap_or(0); // Resolved during file loading if 0

    Ok(DevTask {
        id: final_id,
        priority,
        is_completed,
        completion_date,
        creation_date,
        description,
        project,
        status,
        due_date,
        parent_id,
        line_number: line_num,
    })
}

pub fn format_task(task: &DevTask) -> String {
    let mut parts = Vec::new();
    if task.is_completed {
        parts.push("x".to_string());
        if let Some(p) = task.priority {
            parts.push(format!("({})", p));
        }
        if let Some(ref cd) = task.completion_date {
            parts.push(cd.clone());
        } else {
            parts.push(chrono::Local::now().format("%Y-%m-%d").to_string());
        }
    } else if let Some(p) = task.priority {
        parts.push(format!("({})", p));
    }

    parts.push(task.creation_date.clone());
    parts.push(task.description.clone());

    if let Some(ref p) = task.project {
        parts.push(format!("+{}", p));
    }

    parts.push(format!("status:{}", task.status.to_str()));

    if let Some(ref due) = task.due_date {
        parts.push(format!("due:{}", due));
    }

    parts.push(format!("id:{}", task.id));

    if let Some(p_id) = task.parent_id {
        parts.push(format!("parent:{}", p_id));
    }

    parts.join(" ")
}
