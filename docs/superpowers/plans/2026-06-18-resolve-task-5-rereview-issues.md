# Task 5 Re-Review Issues Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve the three CLI issues found in the Task 5 Re-Review.

**Architecture:** Update `src-tauri/src/cli.rs` to handle `--help` and `--version` using clap's built-in exit handler, update completion state sync in transition command, and return error on task not found.

**Tech Stack:** Rust, clap, chrono

## Global Constraints

- Must compile successfully.
- No GUI libraries are available on the compilation system.
- Commit changes to git when completed.

---

### Task 1: Fix Help/Version Flags and Handle Task Not Found / State Consistency

**Files:**
- Modify: `src-tauri/src/cli.rs:51-147`

**Interfaces:**
- Consumes: `Cli::try_parse()` from clap, `TaskStatus`, `DevTask`
- Produces: Updated `handle_cli()` with correct exit and error handling

- [ ] **Step 1: Write the updated implementation in `src-tauri/src/cli.rs`**
  Modify the `handle_cli` function to:
  - Exit using clap's `e.exit()` on parser errors to support `--help` and `--version`.
  - In `Commands::Transition`, sync the completion state of the task when updating status.
  - In `Transition`, `Complete`, and `Remove` command arms, print to stderr and return `Err("Task not found".to_string())` if the task is not found.

- [ ] **Step 2: Commit changes**
  Run git commands to commit the changes to `src-tauri/src/cli.rs`.
  Run: `git add src-tauri/src/cli.rs && git commit -m "fix(cli): correct help/version flags, sync transition state, and return error on task not found"`
