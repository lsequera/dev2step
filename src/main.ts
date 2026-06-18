import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface Task {
  id: number;
  priority: string | null;
  is_completed: boolean;
  completion_date: string | null;
  creation_date: string;
  description: string;
  project: string | null;
  status: 'Icebox' | 'Todo' | 'Progress' | 'Done';
  due_date: string | null;
  parent_id: number | null;
  line_number: number;
}

interface Metrics {
  status_counts: [string, number][];
  avg_cycle_days: number;
}

let allTasks: Task[] = [];
let focusedIndex = -1;
let visibleTasks: Task[] = [];

const WIP_LIMIT_TODO = 5;
const WIP_LIMIT_PROGRESS = 2;

async function init() {
  setupEventListeners();
  await refreshState();
  
  // Setup file watcher update listener
  await listen<Task[]>("todo-updated", (event) => {
    updateDOM(event.payload);
  });
}

async function refreshState() {
  try {
    const tasks = await invoke<Task[]>("get_tasks");
    updateDOM(tasks);
  } catch (err) {
    console.error("Failed to load tasks", err);
  }
}

function updateDOM(tasks: Task[]) {
  allTasks = tasks;
  renderBoard();
  updateMetrics();
  refreshFocus();
}

function renderBoard() {
  const cols = {
    icebox: document.getElementById("col-icebox")!,
    todo: document.getElementById("col-todo")!,
    progress: document.getElementById("col-progress")!,
    done: document.getElementById("col-done")!
  };

  // Clear columns
  Object.values(cols).forEach(col => col.innerHTML = "");

  // Populate tasks
  let counts = { icebox: 0, todo: 0, progress: 0, done: 0 };
  visibleTasks = [];

  allTasks.forEach(task => {
    const statusKey = task.status.toLowerCase() as keyof typeof cols;
    const colEl = cols[statusKey];
    if (!colEl) return;

    counts[statusKey]++;
    visibleTasks.push(task);

    const card = document.createElement("div");
    card.className = `task-card ${task.is_completed ? 'completed' : ''} ${task.status === 'Progress' ? 'in-progress-active' : ''}`;
    card.setAttribute("data-id", task.id.toString());
    card.tabIndex = 0;

    let priBadge = task.priority ? `<span class="task-priority">${task.priority}</span>` : '';
    let projBadge = task.project ? `<span class="task-project">+${task.project}</span>` : '';

    card.innerHTML = `
      <div class="task-header">
        <span class="task-id">#${task.id}</span>
        ${priBadge}
      </div>
      <div class="task-desc">${escapeHtml(task.description)}</div>
      ${projBadge}
    `;

    // Add drag support
    card.draggable = true;
    card.addEventListener("dragstart", (e) => {
      e.dataTransfer?.setData("text/plain", task.id.toString());
    });

    colEl.appendChild(card);
  });

  // Update counts and enforce WIP limits
  document.querySelector('[data-status="icebox"] .count')!.textContent = counts.icebox.toString();
  
  const todoCountEl = document.getElementById("todo-count")!;
  todoCountEl.textContent = `${counts.todo} / ${WIP_LIMIT_TODO}`;
  const todoCol = document.querySelector('[data-status="todo"]')!;
  if (counts.todo > WIP_LIMIT_TODO) {
    todoCol.classList.add("wip-warning");
  } else {
    todoCol.classList.remove("wip-warning");
  }

  const progressCountEl = document.getElementById("progress-count")!;
  progressCountEl.textContent = `${counts.progress} / ${WIP_LIMIT_PROGRESS}`;
  const progressCol = document.querySelector('[data-status="progress"]')!;
  if (counts.progress > WIP_LIMIT_PROGRESS) {
    progressCol.classList.add("wip-warning");
  } else {
    progressCol.classList.remove("wip-warning");
  }

  document.querySelector('[data-status="done"] .count')!.textContent = counts.done.toString();
  setupCardClicks();
}

function setupCardClicks() {
  document.querySelectorAll(".task-card").forEach(card => {
    card.addEventListener("click", () => {
      const id = parseInt(card.getAttribute("data-id") || "0");
      focusedIndex = visibleTasks.findIndex(t => t.id === id);
      refreshFocus();
    });
  });
}

function refreshFocus() {
  document.querySelectorAll(".task-card").forEach(c => c.classList.remove("focused"));
  if (focusedIndex >= 0 && focusedIndex < visibleTasks.length) {
    const activeTask = visibleTasks[focusedIndex];
    const activeCard = document.querySelector(`.task-card[data-id="${activeTask.id}"]`);
    if (activeCard) {
      activeCard.classList.add("focused");
      (activeCard as HTMLElement).focus();
    }
  }
}

async function updateMetrics() {
  try {
    const metrics = await invoke<Metrics>("get_velocity_metrics");
    document.getElementById("metric-cycle-time")!.textContent = `${metrics.avg_cycle_days.toFixed(1)} days`;
    
    const distEl = document.getElementById("metric-distribution")!;
    distEl.innerHTML = "";
    metrics.status_counts.forEach(([status, count]) => {
      const li = document.createElement("li");
      li.innerHTML = `<span>${status}</span><strong>${count}</strong>`;
      distEl.appendChild(li);
    });
  } catch (err) {
    console.error("Failed to load metrics", err);
  }
}

function setupEventListeners() {
  const omnibar = document.getElementById("omnibar") as HTMLInputElement;
  const addBtn = document.getElementById("add-btn")!;
  const toggleMetricsBtn = document.getElementById("toggle-metrics-btn")!;
  const closeMetricsBtn = document.getElementById("close-metrics-btn")!;
  const metricsPanel = document.getElementById("metrics-panel")!;

  addBtn.addEventListener("click", handleAddTask);
  omnibar.addEventListener("keydown", (e) => {
    if (e.key === "Enter") handleAddTask();
  });

  toggleMetricsBtn.addEventListener("click", () => metricsPanel.classList.toggle("open"));
  closeMetricsBtn.addEventListener("click", () => metricsPanel.classList.remove("open"));

  // Drag and drop setup for columns
  document.querySelectorAll(".column").forEach(col => {
    col.addEventListener("dragover", (e) => e.preventDefault());
    col.addEventListener("drop", async (e) => {
      e.preventDefault();
      const dragEvent = e as DragEvent;
      const id = parseInt(dragEvent.dataTransfer?.getData("text/plain") || "0");
      const status = (col as HTMLElement).getAttribute("data-status") || "";
      if (id > 0 && status) {
        await transitionTask(id, status);
      }
    });
  });

  // Keyboard hotkeys
  document.addEventListener("keydown", async (e) => {
    // Focus search omnibar on '/' or 'N'
    if ((e.key === "/" || e.key === "n" || e.key === "N") && document.activeElement !== omnibar) {
      e.preventDefault();
      omnibar.focus();
      return;
    }

    if (document.activeElement === omnibar) {
      if (e.key === "Escape") {
        omnibar.blur();
      }
      return;
    }

    // Toggles Metrics Panel via Ctrl + M
    if (e.ctrlKey && (e.key === "m" || e.key === "M")) {
      e.preventDefault();
      metricsPanel.classList.toggle("open");
      return;
    }

    // Navigation
    if (visibleTasks.length === 0) return;

    if (e.key === "ArrowDown" || e.key === "Tab") {
      e.preventDefault();
      focusedIndex = (focusedIndex + 1) % visibleTasks.length;
      refreshFocus();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      focusedIndex = (focusedIndex - 1 + visibleTasks.length) % visibleTasks.length;
      refreshFocus();
    }

    // Action transitions based on selected index
    if (focusedIndex >= 0 && focusedIndex < visibleTasks.length) {
      const task = visibleTasks[focusedIndex];
      if (e.key === "1") {
        await transitionTask(task.id, "icebox");
      } else if (e.key === "2") {
        await transitionTask(task.id, "todo");
      } else if (e.key === "3") {
        await transitionTask(task.id, "progress");
      } else if (e.key === "4") {
        await transitionTask(task.id, "done");
      } else if (e.key === "Delete" || e.key === "Backspace") {
        e.preventDefault();
        if (confirm(`Are you sure you want to delete task #${task.id}?`)) {
          const updated = await invoke<Task[]>("delete_task", { id: task.id });
          focusedIndex = Math.min(focusedIndex, updated.length - 1);
          updateDOM(updated);
        }
      }
    }
  });
}

async function handleAddTask() {
  const omnibar = document.getElementById("omnibar") as HTMLInputElement;
  const text = omnibar.value.trim();
  if (!text) return;

  let priority: string | null = null;
  let project: string | null = null;
  let status = "icebox";

  // Extract details inline (simple CLI-like tags)
  const words = text.split(" ");
  const descWords: string[] = [];

  words.forEach(w => {
    if (w.startsWith("(") && w.endsWith(")") && w.length === 3) {
      priority = w.charAt(1);
    } else if (w.startsWith("+") && w.length > 1) {
      project = w.slice(1);
    } else if (w.startsWith("status:")) {
      status = w.slice(7);
    } else {
      descWords.push(w);
    }
  });

  const desc = descWords.join(" ");

  try {
    const updated = await invoke<Task[]>("add_task", {
      description: desc,
      project,
      priority,
      status
    });
    updateDOM(updated);
    omnibar.value = "";
    omnibar.blur();
  } catch (err) {
    console.error("Failed to add task", err);
  }
}

async function transitionTask(id: number, status: string) {
  try {
    const updated = await invoke<Task[]>("update_task_status", { id, status });
    updateDOM(updated);
  } catch (err) {
    console.error("Failed to transition task", err);
  }
}

function escapeHtml(str: string): string {
  const div = document.createElement("div");
  div.innerText = str;
  return div.innerHTML;
}

window.addEventListener("DOMContentLoaded", init);
